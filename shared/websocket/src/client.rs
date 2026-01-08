use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{info, warn, error};

pub struct WebSocketClient {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    url: String,
    auto_reconnect: bool,
    reconnect_attempts: usize,
    max_reconnect_attempts: Option<usize>,
}

impl WebSocketClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(url)
            .await
            .context("Failed to connect to WebSocket")?;
        
        info!("Connected to WebSocket: {}", url);
        
        Ok(Self {
            ws_stream,
            url: url.to_string(),
            auto_reconnect: false,
            reconnect_attempts: 0,
            max_reconnect_attempts: None,
        })
    }
    
    pub fn builder() -> WebSocketClientBuilder {
        WebSocketClientBuilder::default()
    }
    
    pub async fn send<T: Serialize>(&mut self, message: &T) -> Result<()> {
        let json = serde_json::to_string(message)?;
        self.ws_stream
            .send(Message::Text(json))
            .await
            .context("Failed to send message")?;
        Ok(())
    }
    
    pub async fn receive<T: DeserializeOwned>(&mut self) -> Result<Option<T>> {
        loop {
            match self.ws_stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    let message = serde_json::from_str(&text)?;
                    return Ok(Some(message));
                }
                Some(Ok(Message::Ping(data))) => {
                    self.ws_stream.send(Message::Pong(data)).await?;
                }
                Some(Ok(Message::Close(_))) => {
                    if self.auto_reconnect {
                        self.reconnect().await?;
                        continue;
                    } else {
                        return Ok(None);
                    }
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    if self.auto_reconnect {
                        self.reconnect().await?;
                        continue;
                    } else {
                        return Err(e.into());
                    }
                }
                None => {
                    if self.auto_reconnect {
                        self.reconnect().await?;
                        continue;
                    } else {
                        return Ok(None);
                    }
                }
                _ => continue,
            }
        }
    }
    
    async fn reconnect(&mut self) -> Result<()> {
        if let Some(max) = self.max_reconnect_attempts {
            if self.reconnect_attempts >= max {
                return Err(anyhow::anyhow!("Max reconnect attempts reached"));
            }
        }
        
        self.reconnect_attempts += 1;
        let backoff = Duration::from_secs(2u64.pow(self.reconnect_attempts.min(5) as u32));
        
        warn!("Reconnecting in {:?} (attempt {})", backoff, self.reconnect_attempts);
        tokio::time::sleep(backoff).await;
        
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .context("Failed to reconnect")?;
        
        self.ws_stream = ws_stream;
        self.reconnect_attempts = 0; // Reset on successful connection
        
        info!("Reconnected successfully");
        
        Ok(())
    }
}

#[derive(Default)]
pub struct WebSocketClientBuilder {
    url: Option<String>,
    auto_reconnect: bool,
    max_reconnect_attempts: Option<usize>,
}

impl WebSocketClientBuilder {
    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }
    
    pub fn auto_reconnect(mut self, enabled: bool) -> Self {
        self.auto_reconnect = enabled;
        self
    }
    
    pub fn max_reconnect_attempts(mut self, max: usize) -> Self {
        self.max_reconnect_attempts = Some(max);
        self
    }
    
    pub async fn build(self) -> Result<WebSocketClient> {
        let url = self.url.context("URL is required")?;
        let mut client = WebSocketClient::connect(&url).await?;
        client.auto_reconnect = self.auto_reconnect;
        client.max_reconnect_attempts = self.max_reconnect_attempts;
        Ok(client)
    }
}
