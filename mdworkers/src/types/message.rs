use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;

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

/// Forex quote data
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
}

impl Error for MsgError {}

impl fmt::Display for MsgError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ReadError(e) => write!(f, "Error reading message: {}", e),
            Self::SendError(e) => write!(f, "Error sending message: {}", e),
        }
    }
}

impl fmt::Debug for MsgError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}
