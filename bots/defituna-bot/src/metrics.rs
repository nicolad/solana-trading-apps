use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Metrics {
    total_slots: Arc<AtomicU64>,
    total_trades: Arc<AtomicU64>,
    successful_trades: Arc<AtomicU64>,
    failed_trades: Arc<AtomicU64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            total_slots: Arc::new(AtomicU64::new(0)),
            total_trades: Arc::new(AtomicU64::new(0)),
            successful_trades: Arc::new(AtomicU64::new(0)),
            failed_trades: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_slot(&self, _slot: u64) {
        self.total_slots.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_price(&self, _price: f64) {
        // Can add price metrics here
    }

    pub fn record_trade(&self, success: bool) {
        self.total_trades.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_trades.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_trades.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn get_stats(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_slots: self.total_slots.load(Ordering::Relaxed),
            total_trades: self.total_trades.load(Ordering::Relaxed),
            successful_trades: self.successful_trades.load(Ordering::Relaxed),
            failed_trades: self.failed_trades.load(Ordering::Relaxed),
        }
    }
}

pub struct MetricsSnapshot {
    pub total_slots: u64,
    pub total_trades: u64,
    pub successful_trades: u64,
    pub failed_trades: u64,
}

pub fn init_metrics() -> Metrics {
    Metrics::new()
}
