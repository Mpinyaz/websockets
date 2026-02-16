use crate::config::cfg::Config;
use crate::types::{
    assetclass::AssetClass,
    client::{Client, WsStream},
    message::{MsgError, SubscribeData, SubscribeRequest, WsResponse},
};
use futures_util::{SinkExt, StreamExt};
use serde_json;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
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

        let connection_result: Result<(), MsgError> = loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
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
    match cmd {
        WsCommand::Subscribe(data) => {
            let req = SubscribeRequest {
                event_name: "subscribe".to_string(),
                authorization: api_key.to_string(),
                event_data: data.clone(),
            };

            let payload = serde_json::to_string(&req)
                .map_err(|e| MsgError::SendError(format!("Serialization error: {}", e)))?;

            write
                .send(Message::Text(payload.into()))
                .await
                .map_err(|e| MsgError::SendError(e.to_string()))?;

            info!("✅ Subscribed to {:?}: {:?}", asset_class, data.tickers);
        }
        WsCommand::Unsubscribe(data) => {
            let req = SubscribeRequest {
                event_name: "unsubscribe".to_string(),
                authorization: api_key.to_string(),
                event_data: data.clone(),
            };

            let payload = serde_json::to_string(&req)
                .map_err(|e| MsgError::SendError(format!("Serialization error: {}", e)))?;

            write
                .send(Message::Text(payload.into()))
                .await
                .map_err(|e| MsgError::SendError(e.to_string()))?;

            info!("✅ Unsubscribed from {:?}: {:?}", asset_class, data.tickers);
        }
    }

    Ok(())
}

/// Handle incoming WebSocket message
async fn handle_ws_message(message: Message, asset_class: AssetClass) -> Result<(), MsgError> {
    match message {
        Message::Text(text) => match serde_json::from_str::<WsResponse>(&text) {
            Ok(ws_response) => {
                info!(
                    "Received {:?} message type: {}",
                    asset_class, ws_response.message_type
                );

                if ws_response.message_type == "A" {
                    if let Err(e) = publish_data(ws_response).await {
                        error!("❌ Publish failed for {:?}: {}", asset_class, e);
                    }
                }
            }
            Err(e) => {
                error!("Parse error for {:?}: {}", asset_class, e);
            }
        },
        Message::Binary(data) => {
            let text = String::from_utf8(data.into())
                .map_err(|e| MsgError::ParseError(format!("Binary to UTF-8 error: {}", e)))?;
            match serde_json::from_str::<WsResponse>(&text) {
                Ok(ws_response) => {
                    if ws_response.message_type == "A" {
                        if let Err(e) = publish_data(ws_response).await {
                            error!("❌ Publish failed for {:?}: {}", asset_class, e);
                        }
                    }
                }
                Err(e) => {
                    error!("⚠️ Parse error for {:?}: {}", asset_class, e);
                }
            }
        }
        Message::Close(frame) => {
            info!("🔌 Connection closed for {:?}: {:?}", asset_class, frame);
        }
        Message::Ping(_) => {
            info!("🏓 Ping received for {:?}", asset_class);
        }
        Message::Pong(_) => {
            info!("🏓 Pong received for {:?}", asset_class);
        }
        _ => {}
    }

    Ok(())
}

/// Publish data to RabbitMQ stream
async fn publish_data(payload: WsResponse) -> Result<(), MsgError> {
    use crate::services::streams::get_stream_entities;
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
