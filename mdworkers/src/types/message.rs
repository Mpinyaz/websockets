use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Generic subscribe request for WebSocket feeds
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeRequest {
    pub event_name: String,
    pub authorization: String,
    pub event_data: SubscribeData,
}

/// Generic subscribe data for eventData field
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeData {
    /// Optional, for modifying existing subscriptions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<String>,

    /// Optional threshold level as STRING (e.g., "7")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold_level: Option<String>,

    /// Optional list of tickers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tickers: Option<Vec<String>>,
}

/// Generic WebSocket response
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WsResponse {
    /// "I" = Info, "H" = Heartbeat, "A" = Data
    pub message_type: String,

    /// Service identifier (e.g., "fx" for forex data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,

    /// Can be object or array depending on messageType
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ResponseDetails>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseDetails {
    pub code: i32,
    pub message: String,
}

/// Alpaca WebSocket Message (generic wrapper)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "T")]
pub enum AlpacaMessage {
    #[serde(rename = "success")]
    Success(AlpacaControl),
    #[serde(rename = "error")]
    Error(AlpacaControl),
    #[serde(rename = "subscription")]
    Subscription(AlpacaSubscriptionStatus),
    #[serde(rename = "t")]
    Trade(AlpacaTrade),
    #[serde(rename = "q")]
    Quote(AlpacaQuote),
    #[serde(rename = "b")]
    Bar(AlpacaBar),
    #[serde(rename = "d")]
    DailyBar(Value),
    #[serde(rename = "u")]
    UpdatedBar(Value),
    #[serde(rename = "c")]
    Correction(Value),
    #[serde(rename = "x")]
    Cancel(Value),
    #[serde(rename = "l")]
    Luld(Value),
    #[serde(rename = "s")]
    Status(Value),
    #[serde(rename = "i")]
    Imbalance(Value),
    #[serde(rename = "h")]
    Heartbeat(Value),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlpacaControl {
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlpacaSubscriptionStatus {
    pub trades: Vec<String>,
    pub quotes: Vec<String>,
    pub bars: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlpacaTrade {
    #[serde(rename = "S")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: f64,
    #[serde(rename = "s")]
    pub size: f64,
    #[serde(rename = "t")]
    pub ts: String,
    #[serde(rename = "i")]
    pub trade_id: u64,
    #[serde(rename = "x")]
    pub exchange: String,
    #[serde(rename = "z")]
    pub tape: String,
    #[serde(rename = "c")]
    pub conditions: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlpacaQuote {
    #[serde(rename = "S")]
    pub symbol: String,
    #[serde(rename = "bp")]
    pub bid_price: f64,
    #[serde(rename = "bs")]
    pub bid_size: f64,
    #[serde(rename = "ap")]
    pub ask_price: f64,
    #[serde(rename = "as")]
    pub ask_size: f64,
    #[serde(rename = "t")]
    pub ts: String,
    #[serde(rename = "bx")]
    pub bid_exchange: String,
    #[serde(rename = "ax")]
    pub ask_exchange: String,
    #[serde(rename = "c")]
    pub conditions: Vec<String>,
    #[serde(rename = "z")]
    pub tape: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlpacaBar {
    #[serde(rename = "S")]
    pub symbol: String,
    #[serde(rename = "o")]
    pub open: f64,
    #[serde(rename = "h")]
    pub high: f64,
    #[serde(rename = "l")]
    pub low: f64,
    #[serde(rename = "c")]
    pub close: f64,
    #[serde(rename = "v")]
    pub volume: f64,
    #[serde(rename = "t")]
    pub ts: String,
    #[serde(rename = "n")]
    pub trade_count: u64,
    #[serde(rename = "vw")]
    pub vwap: f64,
}

/// "Authorization"
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlpacaAuthRequest {
    pub action: String,
    pub key: String,
    pub secret: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlpacaSubscribeRequest {
    pub action: String, // "subscribe"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trades: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quotes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bars: Option<Vec<String>>,
}

/// Mkt quote data
/// Array format: [type, ticker, timestamp, bidSize, bidPrice, midPrice, askSize, askPrice]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketData {
    /// "Q" for quote update
    #[serde(rename = "type")]
    pub type_field: String,

    /// e.g., "eurusd"
    pub ticker: String,

    /// ISO datetime
    pub timestamp: String,

    /// Number of units at bid
    pub bid_size: f64,

    /// Current highest bid price
    pub bid_price: f64,

    /// (bidPrice + askPrice) / 2
    pub mid_price: f64,

    /// Current lowest ask price
    pub ask_price: f64,

    /// Number of units at ask
    pub ask_size: f64,
}

pub enum MsgError {
    ReadError(String),
    SendError(String),
    ParseError(String),
}

impl Error for MsgError {}

impl fmt::Display for MsgError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ReadError(e) => write!(f, "Error reading message: {}", e),
            Self::SendError(e) => write!(f, "Error sending message: {}", e),
            Self::ParseError(e) => write!(f, "Error parsing message: {}", e),
        }
    }
}

impl fmt::Debug for MsgError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub fn deserialize_msg(msg: Message) -> Result<WsResponse, MsgError> {
    match msg {
        Message::Text(text) => serde_json::from_str::<WsResponse>(&text)
            .map_err(|e| MsgError::ParseError(format!("JSON parse error: {}", e))),
        Message::Binary(data) => {
            let text = String::from_utf8(data.to_vec())
                .map_err(|e| MsgError::ParseError(format!("Binary to UTF-8 error: {}", e)))?;

            serde_json::from_str::<WsResponse>(&text)
                .map_err(|e| MsgError::ParseError(format!("JSON parse error: {}", e)))
        }
        _ => Err(MsgError::ParseError(
            "Expected text or binary message".to_string(),
        )),
    }
}
