use crate::data::pricing::MarketSnapshot;
use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

/// Shared, lock-protected snapshot updated by all feed tasks.
pub type SharedSnapshot = Arc<RwLock<MarketSnapshot>>;

/// Broadcasts a new snapshot to all subscribers whenever data updates.
pub type SnapshotSender = watch::Sender<MarketSnapshot>;
pub type SnapshotReceiver = watch::Receiver<MarketSnapshot>;

pub struct FeedAggregator {
    pub snapshot: SharedSnapshot,
    pub tx: SnapshotSender,
}

impl FeedAggregator {
    /// Called by each feed task when a price update arrives.
    /// Merges the new price into the snapshot and broadcasts.
    pub async fn update_price(&self, symbol: String, price: Decimal) {
        let mut snap = self.snapshot.write().await;
        snap.prices.insert(symbol, price);
        snap.timestamp = Utc::now();
        let _ = self.tx.send(snap.clone());
    }

    pub async fn update_volatility(&self, symbol: String, vol: Decimal) {
        let mut snap = self.snapshot.write().await;
        snap.volatility.insert(symbol, vol);
    }

    pub async fn update_fx_rate(&self, currency: String, rate: Decimal) {
        let mut snap = self.snapshot.write().await;
        snap.fx_rates.insert(currency, rate);
    }
}
