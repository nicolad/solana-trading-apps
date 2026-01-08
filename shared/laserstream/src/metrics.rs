use prometheus::{Encoder, IntCounter, IntGauge, Registry, TextEncoder};
use std::sync::Arc;

pub struct Metrics {
    pub messages_received: IntCounter,
    pub errors: IntCounter,
    pub reconnections: IntCounter,
    pub clients_connected: IntGauge,
    registry: Registry,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();
        
        let messages_received = IntCounter::new(
            "laserstream_messages_received_total",
            "Total messages received from LaserStream",
        )
        .unwrap();
        
        let errors = IntCounter::new(
            "laserstream_errors_total",
            "Total errors processing messages",
        )
        .unwrap();
        
        let reconnections = IntCounter::new(
            "laserstream_reconnections_total",
            "Total reconnection attempts",
        )
        .unwrap();
        
        let clients_connected = IntGauge::new(
            "laserstream_clients_connected",
            "Number of WebSocket clients connected",
        )
        .unwrap();
        
        registry.register(Box::new(messages_received.clone())).unwrap();
        registry.register(Box::new(errors.clone())).unwrap();
        registry.register(Box::new(reconnections.clone())).unwrap();
        registry.register(Box::new(clients_connected.clone())).unwrap();
        
        Arc::new(Self {
            messages_received,
            errors,
            reconnections,
            clients_connected,
            registry,
        })
    }
    
    pub fn record_message(&self) {
        self.messages_received.inc();
    }
    
    pub fn record_error(&self) {
        self.errors.inc();
    }
    
    pub fn record_reconnection(&self) {
        self.reconnections.inc();
    }
    
    pub fn set_clients(&self, count: i64) {
        self.clients_connected.set(count);
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
