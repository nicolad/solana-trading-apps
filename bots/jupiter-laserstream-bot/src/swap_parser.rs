use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Parsed swap event from Jupiter transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub slot: u64,
    pub timestamp: i64,
    pub signature: String,
}

impl SwapEvent {
    /// Calculate the price from this swap event
    /// Price = output_amount / input_amount (normalized by decimals)
    pub fn calculate_price(&self, input_decimals: u8, output_decimals: u8) -> f64 {
        if self.input_amount == 0 {
            return 0.0;
        }

        let input_normalized = self.input_amount as f64 / 10_f64.powi(input_decimals as i32);
        let output_normalized = self.output_amount as f64 / 10_f64.powi(output_decimals as i32);

        output_normalized / input_normalized
    }

    /// Calculate volume in terms of the quote token
    pub fn calculate_volume(&self, quote_decimals: u8, is_input_quote: bool) -> f64 {
        let amount = if is_input_quote {
            self.input_amount
        } else {
            self.output_amount
        };

        amount as f64 / 10_f64.powi(quote_decimals as i32)
    }
}

/// Jupiter Program IDs
pub mod jupiter_programs {

    /// Jupiter V6 program ID
    pub const JUPITER_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

    /// Jupiter V4 program ID (legacy)
    pub const JUPITER_V4: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

    /// Jupiter DCA program ID
    pub const JUPITER_DCA: &str = "DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M";

    pub fn is_jupiter_program(program_id: &str) -> bool {
        program_id == JUPITER_V6 || program_id == JUPITER_V4 || program_id == JUPITER_DCA
    }
}

/// Parse Jupiter swap events from transaction data
pub struct SwapParser {
    // Token mint addresses for filtering
    pub target_input_mint: Option<String>,
    pub target_output_mint: Option<String>,
}

impl SwapParser {
    pub fn new(target_input_mint: Option<String>, target_output_mint: Option<String>) -> Self {
        Self {
            target_input_mint,
            target_output_mint,
        }
    }

    /// Parse a transaction to extract swap events
    /// This is a simplified parser - in production you'd need to parse the actual instruction data
    pub fn parse_transaction(
        &self,
        _transaction_data: &[u8],
        slot: u64,
        signature: String,
    ) -> Result<Option<SwapEvent>> {
        // TODO: Implement full transaction parsing
        // For now, this is a placeholder that shows the structure

        // In a real implementation, you would:
        // 1. Deserialize the transaction
        // 2. Find Jupiter program instructions
        // 3. Parse the instruction data to extract swap parameters
        // 4. Extract pre/post token balances to calculate amounts

        debug!("Parsing transaction {} at slot {}", signature, slot);

        // Placeholder - return None for now
        Ok(None)
    }

    /// Parse account update to detect swap (simplified approach)
    /// This looks at token account balance changes
    pub fn parse_account_update(
        &self,
        pubkey: &str,
        lamports: u64,
        owner: &str,
        _slot: u64,
    ) -> Option<SwapEvent> {
        // Check if this is a token account (owned by Token Program)
        const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

        if owner != TOKEN_PROGRAM {
            return None;
        }

        // In a real implementation, you would:
        // 1. Track token account states
        // 2. Detect balance changes
        // 3. Correlate changes to identify swaps
        // 4. Extract swap amounts from balance deltas

        debug!(
            "Account update: {} (owner: {}, lamports: {})",
            pubkey, owner, lamports
        );

        None
    }

    /// Check if a swap event matches our target token pair
    pub fn matches_target(&self, event: &SwapEvent) -> bool {
        let input_matches = self
            .target_input_mint
            .as_ref()
            .map(|mint| mint == &event.input_mint)
            .unwrap_or(true);

        let output_matches = self
            .target_output_mint
            .as_ref()
            .map(|mint| mint == &event.output_mint)
            .unwrap_or(true);

        // Also check reverse direction (sell instead of buy)
        let reverse_input_matches = self
            .target_output_mint
            .as_ref()
            .map(|mint| mint == &event.input_mint)
            .unwrap_or(true);

        let reverse_output_matches = self
            .target_input_mint
            .as_ref()
            .map(|mint| mint == &event.output_mint)
            .unwrap_or(true);

        (input_matches && output_matches) || (reverse_input_matches && reverse_output_matches)
    }
}

/// Token decimals lookup (common tokens)
pub fn get_token_decimals(mint: &str) -> u8 {
    match mint {
        // SOL
        "So11111111111111111111111111111111111111112" => 9,
        // USDC
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => 6,
        // USDT
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => 6,
        // BONK
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => 5,
        // JUP
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN" => 6,
        // mSOL
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" => 9,
        // jitoSOL
        "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn" => 9,
        // Default
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_event_price_calculation() {
        let event = SwapEvent {
            input_mint: "SOL".to_string(),
            output_mint: "USDC".to_string(),
            input_amount: 1_000_000_000, // 1 SOL (9 decimals)
            output_amount: 100_000_000,  // 100 USDC (6 decimals)
            slot: 12345,
            timestamp: 0,
            signature: "test".to_string(),
        };

        let price = event.calculate_price(9, 6);
        assert_eq!(price, 100.0); // 1 SOL = 100 USDC
    }

    #[test]
    fn test_swap_parser_target_matching() {
        let parser = SwapParser::new(Some("SOL".to_string()), Some("USDC".to_string()));

        let event = SwapEvent {
            input_mint: "SOL".to_string(),
            output_mint: "USDC".to_string(),
            input_amount: 1_000_000_000,
            output_amount: 100_000_000,
            slot: 12345,
            timestamp: 0,
            signature: "test".to_string(),
        };

        assert!(parser.matches_target(&event));

        // Test reverse direction
        let reverse_event = SwapEvent {
            input_mint: "USDC".to_string(),
            output_mint: "SOL".to_string(),
            input_amount: 100_000_000,
            output_amount: 1_000_000_000,
            slot: 12345,
            timestamp: 0,
            signature: "test".to_string(),
        };

        assert!(parser.matches_target(&reverse_event));
    }

    #[test]
    fn test_jupiter_program_detection() {
        use jupiter_programs::*;

        assert!(is_jupiter_program(JUPITER_V6));
        assert!(is_jupiter_program(JUPITER_V4));
        assert!(is_jupiter_program(JUPITER_DCA));
        assert!(!is_jupiter_program("11111111111111111111111111111111"));
    }
}
