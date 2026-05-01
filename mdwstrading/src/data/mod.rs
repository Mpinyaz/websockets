use chrono::{DateTime, Utc};
use mdcore::AssetClass;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;

pub mod feed;
pub mod indicators;
pub mod pricing;

#[derive(Debug, Clone, PartialEq)]
pub enum OptionType {
    Call,
    Put,
}

#[derive(Debug, Clone)]
pub struct OptionContract {
    pub underlying: String,
    pub asset_class: AssetClass,
    pub option_type: OptionType,
    // K
    pub strike: Decimal,
    pub expiry: DateTime<Utc>,
    //Spot price - current underlying price
    pub spot: Decimal,

    // Implied Volatility (annualized)
    pub vol: Decimal,
    // Domestic Rate
    pub risk_free_rate: Decimal,
    pub foreign_rate: Decimal,
    // Continous dividend (equity)
    pub dividend_yield: Decimal,
}

impl OptionContract {
    /// Time to expiry in years
    pub fn time_to_expiry(&self) -> Decimal {
        let now = Utc::now();
        let secs = (self.expiry - now).num_seconds().max(0);
        Decimal::from(secs) / Decimal::from(31_557_600) // 365.25 days
    }
}

#[derive(Debug, Clone, Default)]
pub struct Greeks {
    /// Rate of change of option price with respect to spot price.
    /// Range: [0, 1] for calls, [-1, 0] for puts.
    pub delta: Decimal,

    /// Rate of change of delta with respect to spot price (curvature).
    /// Always positive for long options.
    pub gamma: Decimal,

    /// Sensitivity to a 1% move in implied volatility.
    /// Always positive for long options.
    pub vega: Decimal,

    /// Daily time decay — option value lost per calendar day.
    /// Always negative for long options.
    pub theta: Decimal,

    /// Sensitivity to a 1% move in the risk-free rate.
    /// Positive for calls, negative for puts.
    pub rho: Decimal,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ohlcv {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}
