use crate::{config::Config, types::assetclass::AssetClass};
use anyhow::Result;
use futures_util::SinkExt;
use std::error::Error;
use std::fmt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{error, info};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Client {
    pub api_key: String,
    pub fx_ws: Option<WsStream>,
    pub crypto_ws: Option<WsStream>,
    pub equity_ws: Option<WsStream>,
}

pub enum ClientError {
    InternalServerError,
    ConnectionFailed(String),
    InvalidResponse(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InternalServerError => write!(f, "Internal server error occurred"),
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl fmt::Debug for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error for ClientError {}

impl Client {
    pub async fn connect(cfg: &Config, ac: AssetClass) -> Result<WsStream, ClientError> {
        let url = match ac {
            AssetClass::Crypto | AssetClass::Forex => format!("{}/{}", cfg.data_url, ac),
            AssetClass::Equity => cfg.alpaca_url.clone(),
        };
        info!("Connecting to WebSocket at: {}", url);

        let (mut ws_stream, response) = connect_async(&url)
            .await
            .map_err(|err| ClientError::ConnectionFailed(err.to_string()))?;

        info!(
            "WebSocket connection established for {:?}. Response: {:?}",
            ac,
            response.status()
        );

        if ac == AssetClass::Equity {
            info!("Sending Alpaca auth payload for Equity connection");
            let auth_payload = serde_json::json!({
                "action": "auth",
                "key": cfg.alpaca_key,
                "secret": cfg.alpaca_secret
            });

            ws_stream
                .send(Message::Text(auth_payload.to_string().into()))
                .await
                .map_err(|err| {
                    ClientError::ConnectionFailed(format!("Failed to send auth: {}", err))
                })?;
        }

        Ok(ws_stream)
    }

    pub async fn new(cfg: &Config) -> Result<Self, ClientError> {
        info!("Initializing Client with all WebSocket connections");

        let fx_ws = match Self::connect(cfg, AssetClass::Forex).await {
            Ok(ws) => Some(ws),
            Err(e) => {
                error!("Failed to connect to Forex WebSocket: {}", e);
                None
            }
        };

        let crypto_ws = match Self::connect(cfg, AssetClass::Crypto).await {
            Ok(ws) => Some(ws),
            Err(e) => {
                error!("Failed to connect to Crypto WebSocket: {}", e);
                None
            }
        };

        let equity_ws = match Self::connect(cfg, AssetClass::Equity).await {
            Ok(ws) => Some(ws),
            Err(e) => {
                error!("Failed to connect to Equity WebSocket: {}", e);
                None
            }
        };

        Ok(Client {
            api_key: cfg.data_api_key.clone(),
            fx_ws,
            crypto_ws,
            equity_ws,
        })
    }
}
