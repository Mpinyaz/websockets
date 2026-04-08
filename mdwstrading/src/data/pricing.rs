use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub timestamp: DateTime<Utc>,
    pub prices: HashMap<String, Decimal>,
    pub spreads: HashMap<String, Decimal>,

    // ── Volatility ────────────────────────────────────────────────
    /// Annualised historical volatility per symbol (e.g. 0.25 = 25% vol).
    /// Used by RiskParity and Kelly strategies.
    /// Typically a 20-day or 30-day rolling window.
    pub volatility: HashMap<String, Decimal>,

    /// Average True Range per symbol — absolute price units.
    /// Used for stop placement and Kelly loss estimation.
    pub atr: HashMap<String, Decimal>,

    // ── Correlation ───────────────────────────────────────────────
    /// Pairwise correlation matrix: (symbol_a, symbol_b) → [-1.0, 1.0].
    /// Used by mean-variance optimisation and risk parity with correlation.
    /// Keys are always sorted: ("AAPL", "BTC") not ("BTC", "AAPL").
    pub correlations: HashMap<(String, String), Decimal>,

    // ── FX Rates ──────────────────────────────────────────────────
    /// All rates expressed as units of base_currency per 1 unit of foreign.
    /// e.g. if base is USD: "EUR" → 1.08, "GBP" → 1.26, "BTC" → 67320.0
    pub fx_rates: HashMap<String, Decimal>,

    /// The portfolio's reporting currency.
    pub base_currency: String,

    // ── Liquidity ─────────────────────────────────────────────────
    /// 30-day average daily volume per symbol, in native units.
    /// Used to cap order size to avoid excessive market impact.
    pub avg_daily_volume: HashMap<String, Decimal>,

    // ── Signal Inputs ─────────────────────────────────────────────
    /// Momentum scores per symbol: typically 12-1 month return.
    /// Range is unconstrained — strategies normalise as needed.
    pub momentum: HashMap<String, Decimal>,

    /// Fundamental scores per symbol (equity only).
    /// e.g. P/E z-score, earnings revision, analyst rating.
    pub fundamental_scores: HashMap<String, Decimal>,

    // ── Market Regime ─────────────────────────────────────────────
    pub regime: MarketRegime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarketRegime {
    RiskOn,   // low vol, trending, equities and crypto bid
    RiskOff,  // flight to safety, FX carry unwinds
    HighVol,  // VIX > 30 equivalent — reduce gross leverage
    Trending, // strong directional momentum across classes
    Ranging,  // mean-reverting, low momentum signal quality
}

impl MarketSnapshot {
    /// Safe price lookup — returns None rather than panicking on missing symbol.
    pub fn price(&self, symbol: &str) -> Option<Decimal> {
        self.prices.get(symbol).copied()
    }

    /// Convert a notional value in a symbol's quote currency to base currency.
    pub fn to_base(&self, amount: Decimal, currency: &str) -> Decimal {
        if currency == self.base_currency {
            return amount;
        }
        let rate = self.fx_rates.get(currency).copied().unwrap_or(Decimal::ONE);
        amount * rate
    }

    /// Retrieve correlation between two symbols (order-insensitive).
    pub fn correlation(&self, a: &str, b: &str) -> Option<Decimal> {
        let key = if a < b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        };
        self.correlations.get(&key).copied()
    }

    /// Returns true if volume allows a position of this notional size
    /// without exceeding `max_adv_pct` of average daily volume.
    pub fn is_liquid(
        &self,
        symbol: &str,
        notional: Decimal,
        price: Decimal,
        max_adv_pct: Decimal,
    ) -> bool {
        let adv = self
            .avg_daily_volume
            .get(symbol)
            .copied()
            .unwrap_or(Decimal::ZERO);
        if adv == Decimal::ZERO {
            return false;
        }
        let qty = notional / price;
        qty <= adv * max_adv_pct
    }
}
