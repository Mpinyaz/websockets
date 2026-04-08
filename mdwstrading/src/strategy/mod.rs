use crate::data::MarketSnapshot;
use crate::Asset;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Every allocation strategy implements this trait.
/// It is pure — no I/O, no side effects.
pub trait AllocationStrategy: Send + Sync {
    fn name(&self) -> &str;

    /// Returns a symbol → target weight map.
    /// Weights should sum to 1.0 for a fully invested portfolio,
    /// or less than 1.0 if you want a cash buffer.
    fn compute_weights(
        &self,
        assets: &[Asset],
        snapshot: &MarketSnapshot,
    ) -> HashMap<String, Decimal>;
}
