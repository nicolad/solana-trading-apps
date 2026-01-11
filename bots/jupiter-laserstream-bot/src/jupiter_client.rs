use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

/// Jupiter Price API v2 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterPriceResponse {
    pub data: HashMap<String, TokenPrice>,
    #[serde(rename = "timeTaken")]
    pub time_taken: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub id: String,
    #[serde(rename = "mintSymbol")]
    pub mint_symbol: Option<String>,
    #[serde(rename = "vsToken")]
    pub vs_token: String,
    #[serde(rename = "vsTokenSymbol")]
    pub vs_token_symbol: String,
    pub price: f64,
}

/// Jupiter Quote API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<PlatformFee>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RoutePlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    #[serde(rename = "feeBps")]
    pub fee_bps: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlan {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    pub label: Option<String>,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
}

/// Jupiter Swap API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
}

pub struct JupiterClient {
    client: Client,
    base_url: String,
    price_api_url: String,
}

impl JupiterClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: "https://quote-api.jup.ag/v6".to_string(),
            price_api_url: "https://price.jup.ag/v4".to_string(),
        }
    }

    /// Get current price for a token pair using Jupiter Price API
    /// Returns price as output_token per input_token
    pub async fn get_price(
        &self,
        input_mint: &str,
        output_mint: &str,
    ) -> Result<f64> {
        let url = format!(
            "{}/price?ids={}&vsToken={}",
            self.price_api_url, input_mint, output_mint
        );

        debug!("Fetching price from Jupiter: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch price from Jupiter")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Jupiter price API error: {} - {}", status, text);
        }

        let price_response: JupiterPriceResponse = response
            .json()
            .await
            .context("Failed to parse Jupiter price response")?;

        // Get the price for the input mint
        let price = price_response
            .data
            .get(input_mint)
            .map(|p| p.price)
            .context("Price not found in response")?;

        debug!("Jupiter price: {} {} per {}", price, output_mint, input_mint);

        Ok(price)
    }

    /// Get a quote for swapping tokens
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuoteResponse> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.base_url, input_mint, output_mint, amount, slippage_bps
        );

        debug!("Fetching quote from Jupiter: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch quote from Jupiter")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Jupiter quote API error: {} - {}", status, text);
        }

        let quote: JupiterQuoteResponse = response
            .json()
            .await
            .context("Failed to parse Jupiter quote response")?;

        info!(
            "Jupiter quote: {} {} -> {} {} (impact: {}%)",
            amount,
            input_mint.chars().take(8).collect::<String>(),
            quote.out_amount,
            output_mint.chars().take(8).collect::<String>(),
            quote.price_impact_pct
        );

        Ok(quote)
    }

    /// Get swap transaction for a quote
    pub async fn get_swap_transaction(
        &self,
        quote: &JupiterQuoteResponse,
        user_public_key: &str,
        wrap_unwrap_sol: bool,
    ) -> Result<JupiterSwapResponse> {
        let url = format!("{}/swap", self.base_url);

        let payload = serde_json::json!({
            "quoteResponse": quote,
            "userPublicKey": user_public_key,
            "wrapAndUnwrapSol": wrap_unwrap_sol,
            "computeUnitPriceMicroLamports": "auto",
        });

        debug!("Requesting swap transaction from Jupiter");

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to request swap transaction from Jupiter")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Jupiter swap API error: {} - {}", status, text);
        }

        let swap_response: JupiterSwapResponse = response
            .json()
            .await
            .context("Failed to parse Jupiter swap response")?;

        Ok(swap_response)
    }

    /// Calculate price from a quote (output amount / input amount)
    pub fn calculate_price_from_quote(
        &self,
        quote: &JupiterQuoteResponse,
        input_decimals: u8,
        output_decimals: u8,
    ) -> f64 {
        let in_amount: u64 = quote.in_amount.parse().unwrap_or(0);
        let out_amount: u64 = quote.out_amount.parse().unwrap_or(0);

        if in_amount == 0 {
            return 0.0;
        }

        // Normalize to human-readable amounts
        let in_amount_normalized = in_amount as f64 / 10_f64.powi(input_decimals as i32);
        let out_amount_normalized = out_amount as f64 / 10_f64.powi(output_decimals as i32);

        // Price = output / input
        out_amount_normalized / in_amount_normalized
    }
}

impl Default for JupiterClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_price() {
        let client = JupiterClient::new();
        
        // SOL/USDC price
        let sol_mint = "So11111111111111111111111111111111111111112";
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        
        let result = client.get_price(sol_mint, usdc_mint).await;
        assert!(result.is_ok());
        
        let price = result.unwrap();
        assert!(price > 0.0);
        println!("SOL/USDC price: ${}", price);
    }

    #[tokio::test]
    async fn test_get_quote() {
        let client = JupiterClient::new();
        
        // Quote for 0.1 SOL -> USDC
        let sol_mint = "So11111111111111111111111111111111111111112";
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let amount = 100_000_000; // 0.1 SOL (9 decimals)
        
        let result = client.get_quote(sol_mint, usdc_mint, amount, 50).await;
        assert!(result.is_ok());
        
        let quote = result.unwrap();
        println!("Quote: {} SOL -> {} USDC", amount, quote.out_amount);
    }
}
