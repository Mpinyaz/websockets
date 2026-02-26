use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use influxdb::Client;
use polars::error::PolarsError;
use redis::aio::ConnectionManager;
use redis::RedisError;
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeJsonError;
use serde_json::Value;
use snowflake_me::Error as SnowflakeError;
use std::{fmt, str::FromStr};

use color_eyre::eyre::Result;
use ratatui::prelude::*;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum JobEvent {
    Processing,
    Completed { result: Value },
    Failed { error: String },
}

// New AssetClass enum for validation
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum AssetClass {
    Crypto,
    Forex,
    Equity,
}

impl fmt::Display for AssetClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetClass::Crypto => write!(f, "crypto"),
            AssetClass::Forex => write!(f, "forex"),
            AssetClass::Equity => write!(f, "equity"),
        }
    }
}

impl FromStr for AssetClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "crypto" => Ok(AssetClass::Crypto),
            "forex" => Ok(AssetClass::Forex),
            "equity" => Ok(AssetClass::Equity),
            _ => Err(format!("Unknown AssetClass: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebAppState {
    pub db: Client,
    pub redis: ConnectionManager,
}

#[derive(Debug)]
pub enum ApiError {
    Influx(influxdb::Error),
    Polars(PolarsError),
    Json(SerdeJsonError),
    Snowflake(SnowflakeError),
    Redis(RedisError),
    BadRequest(String),
}

impl From<influxdb::Error> for ApiError {
    fn from(err: influxdb::Error) -> Self {
        ApiError::Influx(err)
    }
}

impl From<PolarsError> for ApiError {
    fn from(err: PolarsError) -> Self {
        ApiError::Polars(err)
    }
}

impl From<SerdeJsonError> for ApiError {
    fn from(err: SerdeJsonError) -> Self {
        ApiError::Json(err)
    }
}

impl From<SnowflakeError> for ApiError {
    // Added this block
    fn from(err: SnowflakeError) -> Self {
        ApiError::Snowflake(err)
    }
}

impl From<String> for ApiError {
    fn from(err: String) -> Self {
        ApiError::BadRequest(err)
    }
}

impl From<RedisError> for ApiError {
    fn from(err: RedisError) -> Self {
        ApiError::Redis(err)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::Influx(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("InfluxDB error: {}", err),
            ),
            ApiError::Polars(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Polars error: {}", err),
            ),
            ApiError::Json(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("JSON serialization/deserialization error: {}", err),
            ),
            ApiError::Snowflake(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Snowflake error: {}", err),
            ),
            ApiError::Redis(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Redis error: {}", err),
            ),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, format!("Bad request: {}", msg)),
        };
        (status, error_message).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Influx(err) => write!(f, "InfluxDB error: {}", err),
            ApiError::Polars(err) => write!(f, "Polars error: {}", err),
            ApiError::Json(err) => write!(f, "JSON error: {}", err),
            ApiError::Snowflake(err) => write!(f, "Snowflake error: {}", err),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::Redis(err) => write!(f, "Redis error: {}", err),
        }
    }
}

#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeRequest {
    pub asset_class: AssetClass,
    pub tickers: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    pub job_id: String,
    pub asset_class: String,
    pub data: Vec<Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MarketUpdate {
    #[serde(rename = "assetClass")]
    pub asset_class: String,
    pub payload: Payload,
    pub time: String, // The outer message RFC3339 timestamp
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Payload {
    Forex {
        #[serde(rename = "type")]
        update_type: String, // "Q"
        ticker: String,
        timestamp: String,
        #[serde(rename = "bidSize")]
        bid_size: f64,
        #[serde(rename = "bidPrice")]
        bid_price: f64,
        #[serde(rename = "midPrice")]
        mid_price: f64,
        #[serde(rename = "askPrice")]
        ask_price: f64,
        #[serde(rename = "askSize")]
        ask_size: f64,
    },
    Crypto {
        update_type: String, // "T"
        ticker: String,
        date: String,
        exchange: String,
        last_size: f64,
        last_price: f64,
    },
    Equity {
        update_type: String, // "T"
        ticker: String,
        timestamp: String,
        price: f64,
        volume: f64,
    },
}

#[derive(PartialEq)]
pub enum ActiveScreen {
    Dashboard,
    Manage,
}

impl Default for ActiveScreen {
    fn default() -> Self {
        ActiveScreen::Dashboard
    }
}

pub enum ManageAction {
    Subscribe,
    Unsubscribe,
}

// App-specific messages for WebSocket communication
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionPayload {
    asset_class: AssetClass,
    tickers: Vec<String>,
}

#[derive(Serialize)]
struct WsRequest<'a> {
    r#type: &'a str,
    payload: SubscriptionPayload,
}

// Structs for parsing WebSocket confirmation responses
#[derive(Debug, serde::Deserialize)]
pub struct ConfirmationPayload {
    pub asset: String,
    pub status: String,
    pub symbol: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WsResponse {
    #[serde(rename = "type")]
    pub message_type: String, // "subscribe", "unsubscribe"
    pub payload: ConfirmationPayload,
    // from, time can be ignored for now
}

// Enum for messages sent to the main app loop
pub enum AppMessage {
    MarketData(MarketUpdate),
    WsConfirmation(WsResponse),
    ClearStatus,
}

pub struct TuiApp {
    pub forex: HashMap<String, MarketUpdate>,
    pub crypto: HashMap<String, MarketUpdate>,
    pub equity: HashMap<String, MarketUpdate>,
    pub tickers: HashMap<AssetClass, Vec<String>>,
    pub selected_tab: AssetClass,
    pub running: bool,
    pub active_screen: ActiveScreen,
    // Manage screen state
    pub manage_selected_asset_class: AssetClass,
    pub manage_selected_ticker_index: usize,
    pub show_popup: bool,
    pub popup_action: Option<ManageAction>,
    // WebSocket outgoing sender
    pub ws_out_tx: Option<mpsc::UnboundedSender<String>>,
    // WebSocket incoming confirmation display
    pub ws_status_message: String,
    pub ws_status_color: Color,
    pub pending_manage_action: Option<(ManageAction, AssetClass, String)>,
}

impl Default for TuiApp {
    fn default() -> Self {
        TuiApp {
            forex: HashMap::new(),
            crypto: HashMap::new(),
            equity: HashMap::new(),
            tickers: {
                let mut map = HashMap::new();
                map.insert(
                    AssetClass::Crypto,
                    vec![
                        "BTCUSD".to_string(),
                        "DOGEUSD".to_string(),
                        "SOLUSD".to_string(),
                        "ETHUSD".to_string(),
                    ],
                );
                map.insert(
                    AssetClass::Forex,
                    vec![
                        "USDZAR".to_string(),
                        "EURZAR".to_string(),
                        "USDJPY".to_string(),
                    ],
                );
                map.insert(
                    AssetClass::Equity,
                    vec![
                        "GOOGL".to_string(),
                        "AAPL".to_string(),
                        "MSFT".to_string(),
                        "NFLX".to_string(),
                    ],
                );
                map
            },
            selected_tab: AssetClass::Forex,
            running: true,
            active_screen: ActiveScreen::default(),
            manage_selected_asset_class: AssetClass::Crypto,
            manage_selected_ticker_index: 0,
            show_popup: false,
            popup_action: None,
            ws_out_tx: None,
            ws_status_message: String::from("Ready"),
            ws_status_color: Color::Gray,
            pending_manage_action: None,
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            AssetClass::Forex => AssetClass::Crypto,
            AssetClass::Crypto => AssetClass::Equity,
            AssetClass::Equity => AssetClass::Forex,
        };
    }

    pub fn prev_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            AssetClass::Crypto => AssetClass::Forex,
            AssetClass::Forex => AssetClass::Equity,
            AssetClass::Equity => AssetClass::Crypto,
        };
    }

    pub fn next_screen(&mut self) {
        self.active_screen = match self.active_screen {
            ActiveScreen::Dashboard => ActiveScreen::Manage,
            ActiveScreen::Manage => ActiveScreen::Dashboard,
        };
    }

    pub fn prev_screen(&mut self) {
        self.active_screen = match self.active_screen {
            ActiveScreen::Dashboard => ActiveScreen::Manage,
            ActiveScreen::Manage => ActiveScreen::Dashboard,
        };
    }

    pub fn update_market_data(&mut self, data: MarketUpdate) {
        let (map, ticker) = match data.asset_class.as_str() {
            "forex" => (&mut self.forex, get_ticker_from_payload(&data.payload)),
            "crypto" => (&mut self.crypto, get_ticker_from_payload(&data.payload)),
            "equity" => (&mut self.equity, get_ticker_from_payload(&data.payload)),
            _ => return,
        };
        if let Some(ticker_str) = ticker {
            map.insert(ticker_str.to_string(), data);
        }
    }

    // Manage screen helper methods
    pub fn select_next_manage_ticker(&mut self) {
        if let Some(tickers) = self.tickers.get(&self.manage_selected_asset_class) {
            if !tickers.is_empty() {
                self.manage_selected_ticker_index = self
                    .manage_selected_ticker_index
                    .saturating_add(1)
                    .min(tickers.len() - 1);
            }
        }
    }

    pub fn select_prev_manage_ticker(&mut self) {
        if let Some(tickers) = self.tickers.get(&self.manage_selected_asset_class) {
            if !tickers.is_empty() {
                self.manage_selected_ticker_index =
                    self.manage_selected_ticker_index.saturating_sub(1).max(0);
            }
        }
    }

    pub fn trigger_subscribe_popup(&mut self) {
        self.show_popup = true;
        self.popup_action = Some(ManageAction::Subscribe);
    }

    pub fn trigger_unsubscribe_popup(&mut self) {
        self.show_popup = true;
        self.popup_action = Some(ManageAction::Unsubscribe);
    }

    pub fn confirm_manage_action(&mut self) {
        if let Some(action) = self.popup_action.take() {
            // .take() consumes the Option
            let asset_class = self.manage_selected_asset_class.clone();
            let empty_tickers_vec = vec![]; // Longer-lived empty Vec<String>
            let current_tickers_for_class =
                self.tickers.get(&asset_class).unwrap_or(&empty_tickers_vec); // Use longer-lived Vec
            let ticker = current_tickers_for_class
                .get(self.manage_selected_ticker_index)
                .cloned()
                .unwrap_or_else(|| "UNKNOWN_TICKER".to_string());

            let (action_type, payload) = match action {
                ManageAction::Subscribe => (
                    "subscribe",
                    SubscriptionPayload {
                        asset_class: asset_class.clone(),
                        tickers: vec![ticker.clone()],
                    },
                ),
                ManageAction::Unsubscribe => (
                    "unsubscribe",
                    SubscriptionPayload {
                        asset_class: asset_class.clone(),
                        tickers: vec![ticker.clone()],
                    },
                ),
            };

            self.pending_manage_action = Some((action, asset_class, ticker));

            let ws_request = WsRequest {
                r#type: action_type,
                payload,
            };

            if let Some(tx) = &self.ws_out_tx {
                if let Ok(json_message) = serde_json::to_string(&ws_request) {
                    let _ = tx.send(json_message);
                }
            }
        }

        self.show_popup = false;
        self.popup_action = None;
    }

    pub fn cancel_manage_action(&mut self) {
        self.show_popup = false;
        self.popup_action = None;
    }

    pub fn set_ws_status(
        &mut self,
        message: String,
        color: Color,
        tx: mpsc::UnboundedSender<AppMessage>,
    ) {
        self.ws_status_message = message;
        self.ws_status_color = color;

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let _ = tx.send(AppMessage::ClearStatus);
        });
    }
}

fn get_ticker_from_payload(payload: &Payload) -> Option<&String> {
    match payload {
        Payload::Forex { ticker, .. } => Some(ticker),
        Payload::Crypto { ticker, .. } => Some(ticker),
        Payload::Equity { ticker, .. } => Some(ticker),
    }
}
