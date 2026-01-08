use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub host: String,
    pub port: u16,
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: Option<usize>,
    pub ping_interval: Duration,
    pub message_buffer_size: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            reconnect_interval: Duration::from_secs(5),
            max_reconnect_attempts: Some(10),
            ping_interval: Duration::from_secs(30),
            message_buffer_size: 1000,
        }
    }
}
