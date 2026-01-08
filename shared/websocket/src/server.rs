use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{info, warn, error};

type ClientId = usize;
type Clients = Arc<RwLock<HashMap<ClientId, tokio::sync::mpsc::UnboundedSender<Message>>>>;

pub struct WebSocketServer {
    clients: Clients,
    next_client_id: Arc<RwLock<ClientId>>,
    local_addr: SocketAddr,
}

impl WebSocketServer {
    pub async fn bind(addr: &str) -> Result<Arc<Self>> {
        let listener = TcpListener::bind(addr)
            .await
            .context("Failed to bind WebSocket listener")?;
        
        let local_addr = listener.local_addr()?;
        info!("WebSocket server listening on {}", local_addr);
        
        let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
        let next_client_id = Arc::new(RwLock::new(0));
        
        let server = Arc::new(Self {
            clients: clients.clone(),
            next_client_id: next_client_id.clone(),
            local_addr,
        });
        
        // Start accepting connections
        let server_clone = server.clone();
        tokio::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                info!("New WebSocket connection from {}", addr);
                
                let clients = clients.clone();
                let next_id = next_client_id.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_connection(stream, clients, next_id).await {
                        error!("WebSocket connection error: {}", e);
                    }
                });
            }
        });
        
        Ok(server)
    }
    
    async fn handle_connection(
        stream: TcpStream,
        clients: Clients,
        next_id: Arc<RwLock<ClientId>>,
    ) -> Result<()> {
        let ws_stream = accept_async(stream)
            .await
            .context("Failed to accept WebSocket")?;
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        
        // Get client ID
        let client_id = {
            let mut id = next_id.write().await;
            let current_id = *id;
            *id += 1;
            current_id
        };
        
        // Register client
        clients.write().await.insert(client_id, tx);
        info!("Client {} connected", client_id);
        
        // Send messages to client
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });
        
        // Receive messages from client
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Ping(data)) => {
                    if let Some(tx) = clients.read().await.get(&client_id) {
                        let _ = tx.send(Message::Pong(data));
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
        
        // Cleanup
        clients.write().await.remove(&client_id);
        info!("Client {} disconnected", client_id);
        send_task.abort();
        
        Ok(())
    }
    
    pub async fn broadcast<T: Serialize>(&self, message: &T) -> Result<()> {
        let json = serde_json::to_string(message)?;
        let msg = Message::Text(json);
        
        let clients = self.clients.read().await;
        let mut disconnected = Vec::new();
        
        for (id, tx) in clients.iter() {
            if tx.send(msg.clone()).is_err() {
                disconnected.push(*id);
            }
        }
        
        // Remove disconnected clients
        if !disconnected.is_empty() {
            drop(clients);
            let mut clients = self.clients.write().await;
            for id in disconnected {
                clients.remove(&id);
                warn!("Removed disconnected client {}", id);
            }
        }
        
        Ok(())
    }
    
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
    
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}
