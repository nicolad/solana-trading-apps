use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PricePoint {
    pub price: f64,
    pub volume: f64,
    pub timestamp: i64,
}

pub struct PriceTracker {
    prices: VecDeque<PricePoint>,
    max_points: usize,
    update_count: u64,
}

impl PriceTracker {
    pub fn new(lookback_minutes: usize) -> Self {
        // Assume ~1 update per second, keep lookback_minutes worth
        let max_points = lookback_minutes * 60;
        
        Self {
            prices: VecDeque::with_capacity(max_points),
            max_points,
            update_count: 0,
        }
    }
    
    pub fn add_price(&mut self, price: f64, volume: f64, timestamp: i64) {
        self.prices.push_back(PricePoint {
            price,
            volume,
            timestamp,
        });
        
        // Remove old data
        while self.prices.len() > self.max_points {
            self.prices.pop_front();
        }
        
        self.update_count += 1;
    }
    
    pub fn current_price(&self) -> Option<f64> {
        self.prices.back().map(|p| p.price)
    }
    
    pub fn moving_average(&self, minutes: usize) -> Option<f64> {
        if self.prices.is_empty() {
            return None;
        }
        
        let cutoff_time = self.prices.back()?.timestamp - (minutes as i64 * 60);
        
        let relevant_prices: Vec<f64> = self.prices
            .iter()
            .filter(|p| p.timestamp >= cutoff_time)
            .map(|p| p.price)
            .collect();
        
        if relevant_prices.is_empty() {
            return None;
        }
        
        let sum: f64 = relevant_prices.iter().sum();
        Some(sum / relevant_prices.len() as f64)
    }
    
    pub fn volume_weighted_average(&self, minutes: usize) -> Option<f64> {
        if self.prices.is_empty() {
            return None;
        }
        
        let cutoff_time = self.prices.back()?.timestamp - (minutes as i64 * 60);
        
        let mut total_value = 0.0;
        let mut total_volume = 0.0;
        
        for point in self.prices.iter().filter(|p| p.timestamp >= cutoff_time) {
            total_value += point.price * point.volume;
            total_volume += point.volume;
        }
        
        if total_volume == 0.0 {
            return None;
        }
        
        Some(total_value / total_volume)
    }
    
    pub fn price_change_percent(&self, minutes: usize) -> Option<f64> {
        let current = self.current_price()?;
        let previous = self.moving_average(minutes)?;
        
        Some((current - previous) / previous * 100.0)
    }
    
    pub fn volatility(&self, minutes: usize) -> Option<f64> {
        if self.prices.is_empty() {
            return None;
        }
        
        let cutoff_time = self.prices.back()?.timestamp - (minutes as i64 * 60);
        
        let relevant_prices: Vec<f64> = self.prices
            .iter()
            .filter(|p| p.timestamp >= cutoff_time)
            .map(|p| p.price)
            .collect();
        
        if relevant_prices.len() < 2 {
            return None;
        }
        
        // Calculate standard deviation
        let mean = relevant_prices.iter().sum::<f64>() / relevant_prices.len() as f64;
        let variance = relevant_prices
            .iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f64>() / relevant_prices.len() as f64;
        
        Some(variance.sqrt())
    }
    
    pub fn update_count(&self) -> u64 {
        self.update_count
    }
    
    pub fn has_sufficient_data(&self, minutes: usize) -> bool {
        if self.prices.is_empty() {
            return false;
        }
        
        let cutoff_time = self.prices.back().unwrap().timestamp - (minutes as i64 * 60);
        let data_points = self.prices.iter().filter(|p| p.timestamp >= cutoff_time).count();
        
        // Require at least 30% of expected data points (assuming ~1/sec)
        data_points >= (minutes * 60 * 30 / 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_price_tracker() {
        let mut tracker = PriceTracker::new(5); // 5 minutes
        
        let now = chrono::Utc::now().timestamp();
        
        tracker.add_price(100.0, 10.0, now);
        tracker.add_price(101.0, 20.0, now + 60);
        tracker.add_price(102.0, 15.0, now + 120);
        
        assert_eq!(tracker.current_price(), Some(102.0));
        assert!((tracker.moving_average(2).unwrap() - 101.5).abs() < 0.01);
    }
}
