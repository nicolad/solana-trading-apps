use prometheus::{Encoder, IntCounter, IntGauge, Registry, TextEncoder};
use std::sync::Arc;

pub struct Metrics {
    pub price_updates: IntCounter,
    pub trades_executed: IntCounter,
    pub trades_failed: IntCounter,
    pub current_price_cents: IntGauge,
    registry: Registry,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();
        
        let price_updates = IntCounter::new(
            "price_updates_total",
            "Total price updates received",
        )
        .unwrap();
        
        let trades_executed = IntCounter::new(
            "trades_executed_total",
            "Total successful trades",
        )
        .unwrap();
        
        let trades_failed = IntCounter::new(
            "trades_failed_total",
            "Total failed trades",
        )
        .unwrap();
        
        let current_price_cents = IntGauge::new(
            "current_price_cents",
            "Current price in cents",
        )
        .unwrap();
        
        registry.register(Box::new(price_updates.clone())).unwrap();
        registry.register(Box::new(trades_executed.clone())).unwrap();
        registry.register(Box::new(trades_failed.clone())).unwrap();
        registry.register(Box::new(current_price_cents.clone())).unwrap();
        
        Arc::new(Self {
            price_updates,
            trades_executed,
            trades_failed,
            current_price_cents,
            registry,
        })
    }
    
    pub fn record_price_update(&self) {
        self.price_updates.inc();
    }
    
    pub fn record_trade(&self, success: bool) {
        if success {
            self.trades_executed.inc();
        } else {
            self.trades_failed.inc();
        }
    }
    
    pub fn set_price(&self, price: f64) {
        self.current_price_cents.set((price * 100.0) as i64);
    }
    
    pub fn export(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

pub fn init_metrics() -> Arc<Metrics> {
    Metrics::new()
}
