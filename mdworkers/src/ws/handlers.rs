use crate::services::streams::create_sub_consumer;
use crate::types::{assetclass::AssetClass, message::MsgError};
use crate::ws::actors::{WsChannels, WsCommand};
use futures_util::StreamExt;
use serde::Deserialize;
use tracing::{error, info};

#[derive(Debug, Deserialize)]
struct StreamSubEvent {
    #[serde(rename = "type")]
    event_type: String,
    payload: SubscribePayload,
}

#[derive(Debug, Deserialize)]
struct SubscribePayload {
    #[serde(rename = "assetClass")]
    asset_class: String,
    tickers: Vec<String>,
}

fn parse_asset_class(s: &str) -> Option<AssetClass> {
    match s.to_lowercase().as_str() {
        "forex" => Some(AssetClass::Forex),
        "crypto" => Some(AssetClass::Crypto),
        "equity" => Some(AssetClass::Equity),
        _ => None,
    }
}

/// Run subscription consumer - receives events from RabbitMQ and sends commands to actors
pub async fn run_sub_consumer(channels: WsChannels) -> Result<(), MsgError> {
    let mut consumer = create_sub_consumer()
        .await
        .map_err(|e| MsgError::ReadError(format!("Failed to create consumer: {}", e)))?;

    info!("📡 Subscription consumer started");

    while let Some(delivery) = consumer.next().await {
        let msg = match delivery {
            Ok(m) => m,
            Err(e) => {
                error!("❌ RabbitMQ consumer error: {}", e);
                continue;
            }
        };

        let payload = match msg.message().data() {
            Some(d) => d,
            None => {
                error!("⚠️ Empty subscription message received");
                continue;
            }
        };

        let event: StreamSubEvent = match serde_json::from_slice(payload) {
            Ok(e) => e,
            Err(e) => {
                error!("⚠️ Failed to parse subscription event: {}", e);
                continue;
            }
        };

        let asset = match parse_asset_class(&event.payload.asset_class) {
            Some(a) => a,
            None => {
                error!("❌ Unknown asset class: {}", event.payload.asset_class);
                continue;
            }
        };

        // Get the appropriate channel for this asset class
        let tx = match channels.get_channel(asset) {
            Some(t) => t,
            None => {
                error!("❌ No channel for {:?}", asset);
                continue;
            }
        };

        // Create subscription data
        let sub_data = crate::types::message::SubscribeData {
            subscription_id: None,
            threshold_level: Some("5".to_string()),
            tickers: Some(event.payload.tickers.clone()),
        };

        // Create command based on event type
        let cmd = match event.event_type.as_str() {
            "subscribe" => WsCommand::Subscribe(sub_data),
            "unsubscribe" => WsCommand::Unsubscribe(sub_data),
            other => {
                error!("❌ Unknown event type: {}", other);
                continue;
            }
        };

        // Send command to actor via channel (non-blocking!)
        if let Err(e) = tx.send(cmd).await {
            error!("❌ Failed to send command to {:?} actor: {}", asset, e);
        } else {
            info!("✅ Sent {:?} command to {:?}", event.event_type, asset);
        }
    }

    info!("📡 Subscription consumer stopped");
    Ok(())
}
