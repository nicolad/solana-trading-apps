use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{info, warn, error};

type ClientId = usize;
type Clients = Arc<RwLock<HashMap<ClientId, tokio::sync::mpsc::UnboundedSender<Message>>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdate {
    pub input_mint: String,
    pub output_mint: String,
    pub price: f64,
    pub volume: u64,
    pub timestamp: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub pubkey: String,
    pub lamports: u64,
    pub owner: String,
    pub slot: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamMessage {
    PriceUpdate(PriceUpdate),
    AccountUpdate(AccountUpdate),
    SlotUpdate { slot: u64, timestamp: i64 },
    Ping,
}

pub struct WebSocketBroadcaster {
    clients: Clients,
    next_client_id: Arc<RwLock<ClientId>>,
}

impl WebSocketBroadcaster {
    pub async fn new(port: u16) -> Result<Arc<Self>> {
        let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
        let next_client_id = Arc::new(RwLock::new(0));
        
        let broadcaster = Arc::new(Self {
            clients: clients.clone(),
            next_client_id: next_client_id.clone(),
        });
        
        // Start WebSocket server
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .context("Failed to bind WebSocket listener")?;
        
        info!("WebSocket server listening on port {}", port);
        
        let broadcaster_clone = broadcaster.clone();
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
        
        Ok(broadcaster)
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
        
        // Receive messages from client (mostly for health checks)
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
    
    pub async fn broadcast(&self, message: StreamMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
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
}
