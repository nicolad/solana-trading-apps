use std::collections::VecDeque;

pub struct PriceTracker {
    prices: VecDeque<PricePoint>,
    max_history_minutes: usize,
}

#[derive(Debug, Clone)]
struct PricePoint {
    price: f64,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl PriceTracker {
    pub fn new(max_history_minutes: usize) -> Self {
        Self {
            prices: VecDeque::new(),
            max_history_minutes,
        }
    }

    pub fn add_price(&mut self, price: f64) {
        let now = chrono::Utc::now();

        // Remove old data points
        let cutoff = now - chrono::Duration::minutes(self.max_history_minutes as i64);
        while let Some(point) = self.prices.front() {
            if point.timestamp < cutoff {
                self.prices.pop_front();
            } else {
                break;
            }
        }

        // Add new price point
        self.prices.push_back(PricePoint {
            price,
            timestamp: now,
        });
    }

    pub fn current_price(&self) -> Option<f64> {
        self.prices.back().map(|p| p.price)
    }

    pub fn len(&self) -> usize {
        self.prices.len()
    }

    pub fn moving_average(&self, minutes: usize) -> Option<f64> {
        if self.prices.is_empty() {
            return None;
        }

        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes as i64);
        let relevant_prices: Vec<f64> = self
            .prices
            .iter()
            .filter(|p| p.timestamp >= cutoff)
            .map(|p| p.price)
            .collect();

        if relevant_prices.is_empty() {
            return None;
        }

        let sum: f64 = relevant_prices.iter().sum();
        Some(sum / relevant_prices.len() as f64)
    }

    pub fn price_change(&self, minutes: usize) -> Option<f64> {
        let current = self.current_price()?;
        let avg = self.moving_average(minutes)?;

        Some((current - avg) / avg)
    }

    pub fn volatility(&self, minutes: usize) -> Option<f64> {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes as i64);
        let relevant_prices: Vec<f64> = self
            .prices
            .iter()
            .filter(|p| p.timestamp >= cutoff)
            .map(|p| p.price)
            .collect();

        if relevant_prices.len() < 2 {
            return None;
        }

        let mean = relevant_prices.iter().sum::<f64>() / relevant_prices.len() as f64;
        let variance = relevant_prices
            .iter()
            .map(|p| {
                let diff = p - mean;
                diff * diff
            })
            .sum::<f64>()
            / relevant_prices.len() as f64;

        Some(variance.sqrt())
    }

    pub fn update_count(&self) -> usize {
        self.prices.len()
    }
}
