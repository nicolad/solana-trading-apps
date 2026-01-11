use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use tracing::{info, warn};

use crate::{AppState, LatestSlot};

// LaserStream SDK (crate name uses hyphen; module uses underscore)
use helius_laserstream::{
    config::LaserstreamConfig,
    grpc::{subscribe_update::UpdateOneof, SubscribeRequest},
};

// In docs.rs, subscribe is listed under client::subscribe
use helius_laserstream::client::subscribe;

pub async fn run_slot_stream(state: AppState) -> anyhow::Result<()> {
    let endpoint = std::env::var("LASERSTREAM_ENDPOINT")
        .context("LASERSTREAM_ENDPOINT is required")?;

    let api_key =
        std::env::var("HELIUS_API_KEY").context("HELIUS_API_KEY is required (x-token)")?;

    // NOTE:
    // LaserStream Rust SDK exposes LaserstreamConfig in the crate.
    // If your version has different field names, adjust here to match docs.rs for `config::LaserstreamConfig`.
    // (Common variants are api_key vs x_token; endpoint as String/Url.)
    let config = LaserstreamConfig {
        endpoint: endpoint.clone(),
        api_key: api_key.clone(),
        ..Default::default()
    };

    // Minimal subscription: slots only
    let request = SubscribeRequest {
        slots: [("slots".to_string(), Default::default())].into(),
        ..Default::default()
    };

    info!("connecting to LaserStream endpoint: {}", endpoint);

    let (mut stream, _handle) = subscribe(config, request)
        .map_err(|e| anyhow!("subscribe() failed: {e:?}"))?;

    while let Some(msg) = stream.next().await {
        let update = match msg {
            Ok(u) => u,
            Err(e) => {
                warn!("stream item error: {:?}", e);
                continue;
            }
        };

        let created_at_rfc3339 = update.created_at.as_ref().and_then(|ts| {
            // prost_types::Timestamp: seconds + nanos
            let secs = ts.seconds;
            let nanos = ts.nanos as u32;
            let dt = DateTime::<Utc>::from_timestamp(secs, nanos)?;
            Some(dt.to_rfc3339())
        });

        if let Some(UpdateOneof::Slot(slot)) = update.update_oneof {
            let latest = LatestSlot {
                slot: slot.slot,
                parent: slot.parent,
                status: format!("{:?}", slot.status()),
                created_at_rfc3339,
            };

            {
                let mut guard = state.latest.write().await;
                *guard = Some(latest.clone());
            }

            // Keep logs sparse but visible
            info!(
                "slot={} parent={:?} status={}",
                latest.slot, latest.parent, latest.status
            );
        }
    }

    Err(anyhow!("LaserStream stream ended unexpectedly"))
}
