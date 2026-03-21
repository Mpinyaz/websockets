use crate::config::Config;
use crate::types::{
    client::{Client, WsStream},
    message::{AlpacaMessage, MsgError, SubscribeData, SubscribeRequest, WsResponse},
};
use futures_util::{SinkExt, StreamExt};
use mdcore::AssetClass;
use serde_json::{self, Value};
use std::collections::HashSet;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{error, info};

#[derive(Debug)]
pub enum WsCommand {
    Subscribe(SubscribeData),
    Unsubscribe(SubscribeData),
}

pub struct WsChannels {
    pub forex_tx: Option<mpsc::Sender<WsCommand>>,
    pub crypto_tx: Option<mpsc::Sender<WsCommand>>,
    pub equity_tx: Option<mpsc::Sender<WsCommand>>,
}

impl WsChannels {
    pub fn get_channel(&self, asset: AssetClass) -> Option<&mpsc::Sender<WsCommand>> {
        match asset {
            AssetClass::Forex => self.forex_tx.as_ref(),
            AssetClass::Crypto => self.crypto_tx.as_ref(),
            AssetClass::Equity => self.equity_tx.as_ref(),
        }
    }
}

/// Spawn WebSocket actors for each asset class
pub async fn spawn_ws_actors(client: Client, cfg: &Config) -> WsChannels {
    let mut forex_tx = None;
    let mut crypto_tx = None;
    let mut equity_tx = None;

    // Spawn Forex actor
    if let Some(fx_ws) = client.fx_ws {
        let (tx, rx) = mpsc::channel(100);
        tokio::spawn(ws_actor(
            fx_ws,
            client.api_key.clone(),
            rx,
            AssetClass::Forex,
            cfg.clone(),
        ));
        forex_tx = Some(tx);
        info!("🎭 Forex actor spawned");
    }

    // Spawn Crypto actor
    if let Some(crypto_ws) = client.crypto_ws {
        let (tx, rx) = mpsc::channel(100);
        tokio::spawn(ws_actor(
            crypto_ws,
            client.api_key.clone(),
            rx,
            AssetClass::Crypto,
            cfg.clone(),
        ));
        crypto_tx = Some(tx);
        info!("🎭 Crypto actor spawned");
    }

    // Spawn Equity actor
    if let Some(equity_ws) = client.equity_ws {
        let (tx, rx) = mpsc::channel(100);
        tokio::spawn(ws_actor(
            equity_ws,
            client.api_key.clone(),
            rx,
            AssetClass::Equity,
            cfg.clone(),
        ));
        equity_tx = Some(tx);
        info!("🎭 Equity actor spawned");
    }

    WsChannels {
        forex_tx,
        crypto_tx,
        equity_tx,
    }
}

/// WebSocket actor - one per asset class
async fn ws_actor(
    initial_conn: WsStream,
    api_key: String,
    mut cmd_rx: mpsc::Receiver<WsCommand>,
    asset_class: AssetClass,
    cfg: Config,
) -> Result<(), MsgError> {
    info!("🎭 Actor started for {:?}", asset_class);

    let mut current_conn_opt = Some(initial_conn);
    let mut active_tickers: HashSet<String> = HashSet::new();

    loop {
        let conn = if let Some(c) = current_conn_opt.take() {
            c
        } else {
            info!("Attempting to establish connection for {:?}", asset_class);
            tokio::time::sleep(Duration::from_secs(5)).await;
            match Client::connect(&cfg, asset_class).await {
                Ok(new_conn) => {
                    info!("✅ Connection established for {:?}", asset_class);
                    new_conn
                }
                Err(reconnect_err) => {
                    error!(
                        "❌ Connection failed for {:?}: {}",
                        asset_class, reconnect_err
                    );
                    continue;
                }
            }
        };

        let (mut write, mut read) = conn.split();

        // Re-subscribe if we have active tickers
        if !active_tickers.is_empty() {
            let tickers: Vec<String> = active_tickers.iter().cloned().collect();
            info!(
                "🔄 Re-subscribing to {} tickers for {:?}",
                tickers.len(),
                asset_class
            );
            let sub_data = SubscribeData {
                tickers: Some(tickers),
                subscription_id: None,
                threshold_level: Some("5".to_string()), // Default threshold
            };
            if let Err(e) = handle_command(
                &mut write,
                &api_key,
                WsCommand::Subscribe(sub_data),
                asset_class,
            )
            .await
            {
                error!(
                    "❌ Failed to re-subscribe for {:?} after reconnect: {}",
                    asset_class, e
                );
            }
        }

        let connection_result: Result<(), MsgError> = loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    // Update tracked tickers
                    match &cmd {
                        WsCommand::Subscribe(data) => {
                            if let Some(tickers) = &data.tickers {
                                for ticker in tickers {
                                    active_tickers.insert(ticker.clone());
                                }
                            }
                        }
                        WsCommand::Unsubscribe(data) => {
                            if let Some(tickers) = &data.tickers {
                                for ticker in tickers {
                                    active_tickers.remove(ticker);
                                }
                            }
                        }
                    }

                    if let Err(e) = handle_command(&mut write, &api_key, cmd, asset_class).await {
                        error!("❌ Command error for {:?}: {}", asset_class, e);
                    }
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(message)) => {
                            if let Err(e) = handle_ws_message(message, asset_class).await {
                                error!("⚠️ Message handling error for {:?}: {}", asset_class, e);
                            }
                        }
                        Some(Err(e)) => {
                            error!("❌ WebSocket error for {:?}: {}", asset_class, e);
                            break Err(MsgError::ReadError(e.to_string()));
                        }
                        None => {
                            info!("🔌 WebSocket stream ended for {:?}", asset_class);
                            break Err(MsgError::ReadError("Stream ended".to_string()));
                        }
                    }
                }
            }
        };

        if let Err(e) = connection_result {
            error!("Connection for {:?} closed due to: {}", asset_class, e);
            // Outer loop will now attempt to establish a new connection
        } else {
            // This branch should ideally not be reached if the inner loop breaks only on errors/None
            // If it does, it means the inner loop completed without error, which is unexpected for a continuous stream.
            // For now, we'll log and break the outer loop.
            info!("🎭 Actor stopping gracefully for {:?}", asset_class);
            break;
        }
    }

    Ok(())
}

/// Handle a command sent to the actor
async fn handle_command(
    write: &mut futures_util::stream::SplitSink<WsStream, Message>,
    api_key: &str,
    cmd: WsCommand,
    asset_class: AssetClass,
) -> Result<(), MsgError> {
    let payload = match &cmd {
        WsCommand::Subscribe(data) => {
            if asset_class == AssetClass::Equity {
                let uppercase_tickers = data
                    .tickers
                    .clone()
                    .map(|tickers| tickers.into_iter().map(|t| t.to_uppercase()).collect());
                let req = crate::types::message::AlpacaSubscribeRequest {
                    action: "subscribe".to_string(),
                    trades: uppercase_tickers.clone(),
                    quotes: uppercase_tickers.clone(),
                    bars: uppercase_tickers,
                };
                serde_json::to_string(&req).map_err(|e| MsgError::SendError(e.to_string()))?
            } else {
                let req = SubscribeRequest {
                    event_name: "subscribe".to_string(),
                    authorization: api_key.to_string(),
                    event_data: data.clone(),
                };
                serde_json::to_string(&req).map_err(|e| MsgError::SendError(e.to_string()))?
            }
        }
        WsCommand::Unsubscribe(data) => {
            if asset_class == AssetClass::Equity {
                let uppercase_tickers = data
                    .tickers
                    .clone()
                    .map(|tickers| tickers.into_iter().map(|t| t.to_uppercase()).collect());
                let req = crate::types::message::AlpacaSubscribeRequest {
                    action: "unsubscribe".to_string(),
                    trades: uppercase_tickers.clone(),
                    quotes: uppercase_tickers.clone(),
                    bars: uppercase_tickers,
                };
                serde_json::to_string(&req).map_err(|e| MsgError::SendError(e.to_string()))?
            } else {
                let req = SubscribeRequest {
                    event_name: "unsubscribe".to_string(),
                    authorization: api_key.to_string(),
                    event_data: data.clone(),
                };
                serde_json::to_string(&req).map_err(|e| MsgError::SendError(e.to_string()))?
            }
        }
    };

    write
        .send(Message::Text(payload.into()))
        .await
        .map_err(|e| MsgError::SendError(e.to_string()))?;

    if let WsCommand::Subscribe(data) = &cmd {
        info!("✅ Subscribed to {:?}: {:?}", asset_class, data.tickers);
    } else if let WsCommand::Unsubscribe(data) = &cmd {
        info!("✅ Unsubscribed from {:?}: {:?}", asset_class, data.tickers);
    }

    Ok(())
}

/// Handle incoming WebSocket message
async fn handle_ws_message(message: Message, asset_class: AssetClass) -> Result<(), MsgError> {
    match message {
        Message::Text(text) => process_text_message(text.to_string(), asset_class).await,
        Message::Binary(data) => {
            let text = String::from_utf8(data.into())
                .map_err(|e| MsgError::ParseError(format!("Binary to UTF-8 error: {}", e)))?;
            process_text_message(text, asset_class).await
        }
        Message::Close(frame) => {
            info!("🔌 Connection closed for {:?}: {:?}", asset_class, frame);
            Ok(())
        }
        Message::Ping(_) => {
            info!("🏓 Ping received for {:?}", asset_class);
            Ok(())
        }
        Message::Pong(_) => {
            info!("🏓 Pong received for {:?}", asset_class);
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn process_text_message(text: String, asset_class: AssetClass) -> Result<(), MsgError> {
    if asset_class == AssetClass::Equity {
        // Alpaca message processing
        if let Ok(alpaca_messages) = serde_json::from_str::<Vec<AlpacaMessage>>(&text) {
            for msg in alpaca_messages {
                match msg {
                    AlpacaMessage::Trade(_) => {
                        let normalized = WsResponse {
                            message_type: "A".to_string(),
                            service: Some("alpaca-iex".to_string()),
                            data: Some(serde_json::to_value(&msg).unwrap_or(Value::Null)), // Serialize the entire enum
                            response: None,
                        };
                        info!("Processing Alpaca trade message: {:?}", normalized);

                        if let Err(e) = publish_data(normalized).await {
                            error!("❌ Publish failed for Alpaca {:?}: {}", asset_class, e);
                        }
                    }
                    AlpacaMessage::Quote(_) => {
                        let normalized = WsResponse {
                            message_type: "A".to_string(),
                            service: Some("alpaca-iex".to_string()),
                            data: Some(serde_json::to_value(&msg).unwrap_or(Value::Null)), // Serialize the entire enum
                            response: None,
                        };
                        info!("Processing Alpaca quote message: {:?}", normalized);

                        if let Err(e) = publish_data(normalized).await {
                            error!("❌ Publish failed for Alpaca {:?}: {}", asset_class, e);
                        }
                    }
                    AlpacaMessage::Bar(_) => {
                        let normalized = WsResponse {
                            message_type: "A".to_string(),
                            service: Some("alpaca-iex".to_string()),
                            data: Some(serde_json::to_value(&msg).unwrap_or(Value::Null)), // Serialize the entire enum
                            response: None,
                        };
                        info!("Processing Alpaca bar message: {:?}", normalized);

                        if let Err(e) = publish_data(normalized).await {
                            error!("❌ Publish failed for Alpaca {:?}: {}", asset_class, e);
                        }
                    }
                    AlpacaMessage::DailyBar(data)
                    | AlpacaMessage::UpdatedBar(data)
                    | AlpacaMessage::Correction(data)
                    | AlpacaMessage::Cancel(data)
                    | AlpacaMessage::Luld(data)
                    | AlpacaMessage::Status(data)
                    | AlpacaMessage::Imbalance(data) => {
                        info!("Received unhandled Alpaca message: {:?}", data);
                    }
                    AlpacaMessage::Success(ctrl) => {
                        info!("Alpaca {:?} success: {}", asset_class, ctrl.msg);
                    }
                    AlpacaMessage::Error(ctrl) => {
                        error!(
                            "Alpaca {:?} error: {} (code: {:?})",
                            asset_class, ctrl.msg, ctrl.code
                        );
                    }
                    AlpacaMessage::Subscription(sub) => {
                        info!("Alpaca {:?} subscription status: {:?}", asset_class, sub);
                    }
                    AlpacaMessage::Heartbeat(_) => {
                        // Heartbeats are frequent, so we don't log them to avoid noise.
                    }
                }
            }
        } else {
            error!(
                "Parse error for {:?} (Alpaca): Could not deserialize. Raw message: {}",
                asset_class, text
            );
        }
    } else {
        // Tiingo message processing
        if let Ok(ws_response) = serde_json::from_str::<WsResponse>(&text) {
            if ws_response.message_type == "A" {
                info!(
                    "Processing {:?} data message: {:?}",
                    asset_class, ws_response
                );
                if let Err(e) = publish_data(ws_response).await {
                    error!("❌ Publish failed for {:?}: {}", asset_class, e);
                }
            } else {
                info!(
                    "Received {:?} message type: {}",
                    asset_class, ws_response.message_type
                );
            }
        } else {
            error!(
                "Parse error for {:?} (Tiingo): Could not deserialize. Raw message: {}",
                asset_class, text
            );
        }
    }

    Ok(())
}

/// Publish data to RabbitMQ stream
async fn publish_data(payload: WsResponse) -> Result<(), MsgError> {
    use crate::streams::get_stream_entities;
    use rabbitmq_stream_client::types::Message as StreamMessage;

    let streams = get_stream_entities()
        .await
        .map_err(|e| MsgError::SendError(format!("Failed to get streams: {}", e)))?;

    let json_bytes = serde_json::to_vec(&payload)
        .map_err(|e| MsgError::SendError(format!("Serialization error: {}", e)))?;

    let msg = StreamMessage::builder()
        .properties()
        .content_type("application/json")
        .message_builder()
        .body(json_bytes)
        .build();

    streams
        .mkt_feed_producer
        .send_with_confirm(msg)
        .await
        .map_err(|e| MsgError::SendError(format!("Stream send error: {}", e)))?;

    Ok(())
}
