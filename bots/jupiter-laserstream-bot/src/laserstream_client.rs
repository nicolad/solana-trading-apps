use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotUpdate {
    pub slot: u64,
    pub timestamp: String,
}

pub struct LaserStreamClient {
    base_url: String,
    client: Client,
}

impl LaserStreamClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: base_url.into(),
            client,
        }
    }

    /// Start the LaserStream subscription
    pub async fn start(&self) -> Result<()> {
        let url = format!("{}/start", self.base_url);
        
        debug!("Starting LaserStream subscription at {}", url);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .context("Failed to send start request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("LaserStream start failed: {} - {}", status, text);
        }

        let text = response.text().await?;
        info!("LaserStream subscription started: {}", text);
        
        Ok(())
    }

    /// Get the latest slot update
    pub async fn get_latest(&self) -> Result<Option<SlotUpdate>> {
        let url = format!("{}/latest", self.base_url);
        
        debug!("Polling LaserStream at {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to get latest slot")?;

        debug!("LaserStream response status: {}", response.status());

        if response.status().is_success() {
            let text = response.text().await?;
            
            // Handle "no data yet" response
            if text.contains("no data yet") {
                debug!("LaserStream response: no data yet");
                return Ok(None);
            }
            
            // Try to parse as JSON
            match serde_json::from_str::<SlotUpdate>(&text) {
                Ok(update) => {
                    debug!("Received slot update: {:?}", update);
                    Ok(Some(update))
                }
                Err(_) => {
                    warn!("Could not parse slot update: {}", text);
                    Ok(None)
                }
            }
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            debug!("LaserStream returned {}: {}", status, text);
            Ok(None)
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to check health")?;

        Ok(response.status().is_success())
    }

    /// Poll for slot updates with a callback
    pub async fn poll_updates<F>(&self, mut callback: F, interval: Duration) -> Result<()>
    where
        F: FnMut(SlotUpdate) -> Result<()>,
    {
        loop {
            match self.get_latest().await {
                Ok(Some(update)) => {
                    if let Err(e) = callback(update) {
                        warn!("Callback error: {}", e);
                    }
                }
                Ok(None) => {
                    debug!("No slot data yet");
                }
                Err(e) => {
                    warn!("Failed to get latest slot: {}", e);
                }
            }

            tokio::time::sleep(interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let client = LaserStreamClient::new("https://laserstream-container.eeeew.workers.dev");
        let result = client.health_check().await;
        assert!(result.is_ok());
    }
}
