use mdcore::AssetClass;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::Asset;

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Long,
    Short,
}

#[derive(Debug, Clone)]
pub struct LeveragedPosition {
    pub asset: Asset,
    pub direction: Direction,
    pub margin_posted: Decimal,
    pub leverage_ratio: Decimal,
    pub notional_value: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub liquidation_price: Decimal,
    pub maintenance_margin_rate: Decimal,
}
fn compute_liquidation_price(
    dir: &Direction,
    entry: Decimal,
    leverage: Decimal,
    mmr: Decimal,
) -> Decimal {
    match dir {
        Direction::Long => entry * (dec!(1) - dec!(1) / leverage + mmr),
        Direction::Short => entry * (dec!(1) + dec!(1) / leverage - mmr),
    }
}

fn asset_maintenance_margin_rate(class: &AssetClass) -> Decimal {
    match class {
        AssetClass::Equity => dec!(0.25),
        AssetClass::Forex => dec!(0.02),
        AssetClass::Crypto => dec!(0.05),
    }
}

impl LeveragedPosition {
    pub fn new(
        asset: Asset,
        direction: Direction,
        margin_posted: Decimal,
        leverage_ratio: Decimal,
        entry_price: Decimal,
    ) -> Self {
        let notional = margin_posted * leverage_ratio;
        let mmr = asset_maintenance_margin_rate(&asset.asset_class);

        let liq_price = compute_liquidation_price(&direction, entry_price, leverage_ratio, mmr);

        LeveragedPosition {
            asset,
            direction,
            margin_posted,
            leverage_ratio,
            notional_value: notional,
            entry_price,
            current_price: entry_price,
            liquidation_price: liq_price,
            maintenance_margin_rate: mmr,
        }
    }

    pub fn quantity(&self) -> Decimal {
        self.notional_value / self.entry_price
    }

    pub fn unrealized_pnl(&self) -> Decimal {
        let qty = self.quantity();

        match self.direction {
            Direction::Long => qty * (self.current_price - self.entry_price),
            Direction::Short => qty * (self.entry_price - self.current_price),
        }
    }

    pub fn current_equity(&self) -> Decimal {
        self.margin_posted + self.unrealized_pnl()
    }

    pub fn margin_ratio(&self) -> Decimal {
        self.current_equity() / self.notional_value
    }

    pub fn is_margin_call(&self) -> bool {
        self.margin_ratio() < self.maintenance_margin_rate
    }

    pub fn effective_leverage(&self) -> Decimal {
        // Recalculates actual leverage as price moves
        self.notional_value / self.current_equity().max(dec!(0.001))
    }
}
