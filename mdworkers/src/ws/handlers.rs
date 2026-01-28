use crate::types::{
    assetclass::AssetClass,
    client::Client,
    client::WsStream,
    message::{MarketData, MsgError, SubscribeData, SubscribeRequest, WsResponse},
};
use anyhow::Result;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde_json::json;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::error;

pub async fn msg_handler<F>(
    mut client: Client,
    asset_class: AssetClass,
    mut on_message: F,
) -> Result<(), MsgError>
where
    F: FnMut(Message) -> Result<()>,
{
    let conn = match asset_class {
        AssetClass::Forex => client.fx_ws.take(),
        AssetClass::Crypto => client.crypto_ws.take(),
        AssetClass::Equity => client.equity_ws.take(),
    }
    .ok_or_else(|| MsgError::ReadError(format!("{:?} not connected", asset_class)))?;
    let (mut write, mut read) = conn.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(message) => {
                if let Err(e) = on_message(message) {
                    error!("Error handling message: {}", e);
                }
            }
            Err(e) => {
                return Err(MsgError::ReadError(format!("WebSocket error: {}", e)));
            }
        }
    }
    Ok(())
}

pub async fn subscribe_data(conn: WsStream, symbols: Vec<String>) -> Result<(), MsgError> {
    let subscribe_req = json!(SubscribeRequest {
        event_name: "subscribe".to_string(),
        authorization: client.api_key,
        event_data: SubscribeData {
            subscription_id: None,
            threshold_level: Some("7".to_string()),
            tickers: Some(symbols),
        },
    });

    conn.send(Message::Text(subscribe_req.to_string().into()))
        .await
        .map_err(|e| MsgError::SendError(e.to_string()))?;

    Ok(())
}
