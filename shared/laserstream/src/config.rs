use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaserStreamConfig {
    pub api_key: String,
    pub endpoint: String,
    pub region: String,
    pub network: Network,
    pub broadcast_port: u16,
    pub commitment_level: CommitmentLevel,
    pub start_slot: Option<u64>,
    pub auto_reconnect: bool,
    pub max_reconnect_attempts: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Devnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitmentLevel {
    Processed,
    Confirmed,
    Finalized,
}

impl LaserStreamConfig {
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("HELIUS_API_KEY").context("HELIUS_API_KEY not set")?;

        let region = env::var("LASERSTREAM_REGION").unwrap_or_else(|_| "ewr".to_string());

        let network = match env::var("LASERSTREAM_NETWORK")
            .unwrap_or_else(|_| "mainnet".to_string())
            .to_lowercase()
            .as_str()
        {
            "devnet" => Network::Devnet,
            _ => Network::Mainnet,
        };

        let endpoint = Self::build_endpoint(&region, &network);

        let broadcast_port = env::var("BROADCAST_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .context("Invalid BROADCAST_PORT")?;

        let commitment_level = match env::var("COMMITMENT_LEVEL")
            .unwrap_or_else(|_| "confirmed".to_string())
            .to_lowercase()
            .as_str()
        {
            "processed" => CommitmentLevel::Processed,
            "finalized" => CommitmentLevel::Finalized,
            _ => CommitmentLevel::Confirmed,
        };

        let start_slot = env::var("START_SLOT").ok().and_then(|s| s.parse().ok());

        let auto_reconnect = env::var("AUTO_RECONNECT")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let max_reconnect_attempts = env::var("MAX_RECONNECT_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse().ok());

        Ok(Self {
            api_key,
            endpoint,
            region,
            network,
            broadcast_port,
            commitment_level,
            start_slot,
            auto_reconnect,
            max_reconnect_attempts,
        })
    }

    fn build_endpoint(region: &str, network: &Network) -> String {
        let network_str = match network {
            Network::Mainnet => "mainnet",
            Network::Devnet => "devnet",
        };

        // Devnet only available in Newark (ewr)
        let region_str = match network {
            Network::Devnet => "ewr",
            Network::Mainnet => region,
        };

        format!(
            "https://laserstream-{}-{}.helius-rpc.com",
            network_str, region_str
        )
    }
}

impl CommitmentLevel {
    pub fn to_grpc(&self) -> yellowstone_grpc_proto::prelude::CommitmentLevel {
        match self {
            CommitmentLevel::Processed => {
                yellowstone_grpc_proto::prelude::CommitmentLevel::Processed
            }
            CommitmentLevel::Confirmed => {
                yellowstone_grpc_proto::prelude::CommitmentLevel::Confirmed
            }
            CommitmentLevel::Finalized => {
                yellowstone_grpc_proto::prelude::CommitmentLevel::Finalized
            }
        }
    }
}
