use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    // Market data
    PriceUpdate {
        input_mint: String,
        output_mint: String,
        price: f64,
        volume: u64,
        timestamp: i64,
    },
    
    // Account updates
    AccountUpdate {
        pubkey: String,
        lamports: u64,
        slot: u64,
        timestamp: i64,
    },
    
    // Slot updates
    SlotUpdate {
        slot: u64,
        timestamp: i64,
    },
    
    // Control messages
    Subscribe {
        channels: Vec<String>,
    },
    
    Unsubscribe {
        channels: Vec<String>,
    },
    
    // Health
    Ping,
    Pong,
}
