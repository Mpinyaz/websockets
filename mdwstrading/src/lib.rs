pub mod data;
pub mod portfolio;
pub mod risk;
pub mod strategy;
use chrono::{DateTime, Utc};
use mdcore::AssetClass;
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct Asset {
    pub symbol: String,
    pub asset_class: AssetClass,
    pub base_currency: String,
    pub quote_currency: String,
}

pub trait Priceable {
    fn symbol(&self) -> &str;
    fn asset_class(&self) -> &AssetClass;
    fn pip_size(&self) -> Decimal;
    fn contract_size(&self) -> Decimal;
}

#[derive(Debug, Clone)]
pub struct Position {
    pub asset: Asset,
    pub quantity: Decimal,
    pub avg_entry_price: Decimal,
    pub current_price: Decimal,
    pub opened_at: DateTime<Utc>,
}

impl Position {
    pub fn market_value(&self) -> Decimal {
        self.quantity * self.current_price
    }

    pub fn unrealized_pnl(&self) -> Decimal {
        self.quantity * (self.current_price - self.avg_entry_price)
    }
}
