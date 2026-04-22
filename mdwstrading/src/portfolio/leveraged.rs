use mdcore::AssetClass;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

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
    leverage_ratio: Decimal,
    mmr: Decimal,
) -> Decimal {
    match dir {
        Direction::Long => entry * (dec!(1) - leverage_ratio + mmr),
        Direction::Short => entry * (dec!(1) + leverage_ratio - mmr),
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
        let notional = margin_posted / leverage_ratio;
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

#[derive(Debug, Clone)]
pub struct LeveragePolicy {
    pub max_leverage: Decimal,
    pub leverage_ratio: Decimal,
    pub maintenance_margin_rate: Decimal,
    pub lot_size: Decimal,
}

impl LeveragePolicy {
    pub fn for_equity() -> Self {
        LeveragePolicy {
            max_leverage: dec!(4),
            leverage_ratio: dec!(0.25),
            maintenance_margin_rate: dec!(0.25),
            lot_size: dec!(1),
        }
    }

    pub fn for_forex(max_lev: Decimal) -> Self {
        LeveragePolicy {
            max_leverage: max_lev,
            leverage_ratio: dec!(1) / max_lev,
            maintenance_margin_rate: dec!(0.02),
            lot_size: dec!(100_000), // 1 standard lot
        }
    }

    pub fn for_crypto(max_lev: Decimal, lot_size: Decimal) -> Self {
        LeveragePolicy {
            max_leverage: max_lev,
            leverage_ratio: dec!(1) / max_lev,
            maintenance_margin_rate: dec!(0.05),
            lot_size,
        }
    }

    pub fn max_position_notional(&self, available_margin: Decimal) -> Decimal {
        available_margin / self.leverage_ratio
    }
}

pub struct LeveragedPortfolio {
    pub cash: HashMap<String, Decimal>,
    pub positions: HashMap<String, LeveragedPosition>,
    pub base_currency: String,
    pub target_gross_leverage: Decimal,
    pub max_gross_leverage: Decimal,
}

impl LeveragedPortfolio {
    pub fn nav(&self, fx_rates: &HashMap<String, Decimal>) -> Decimal {
        let cash: Decimal = self
            .cash
            .iter()
            .map(|(ccy, amt)| amt * fx_rates.get(ccy).copied().unwrap_or(Decimal::ONE))
            .sum();

        let pos_equity: Decimal = self
            .positions
            .values()
            .map(|p| {
                let rate = fx_rates
                    .get(&p.asset.base_currency)
                    .copied()
                    .unwrap_or(Decimal::ONE);
                p.current_equity() * rate
            })
            .sum();

        cash + pos_equity
    }

    /// Gross leverage = sum(|notional|) / NAV
    pub fn gross_leverage(&self, fx_rates: &HashMap<String, Decimal>) -> Decimal {
        let nav = self.nav(fx_rates);
        if nav <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        let total_notional: Decimal = self
            .positions
            .values()
            .map(|p| {
                let rate = fx_rates
                    .get(&p.asset.base_currency)
                    .copied()
                    .unwrap_or(Decimal::ONE);
                p.notional_value * rate
            })
            .sum();

        total_notional / nav
    }

    /// Net leverage = (longs - shorts) / NAV  — directional risk
    pub fn net_leverage(&self, fx_rates: &HashMap<String, Decimal>) -> Decimal {
        let nav = self.nav(fx_rates);
        if nav <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        let net_notional: Decimal = self
            .positions
            .values()
            .map(|p| {
                let rate = fx_rates
                    .get(&p.asset.base_currency)
                    .copied()
                    .unwrap_or(Decimal::ONE);
                match p.direction {
                    Direction::Long => p.notional_value * rate,
                    Direction::Short => -p.notional_value * rate,
                }
            })
            .sum();

        net_notional / nav
    }

    /// Per-asset-class leverage breakdown
    pub fn leverage_by_class(&self, fx: &HashMap<String, Decimal>) -> HashMap<String, Decimal> {
        let nav = self.nav(fx);
        let mut by_class: HashMap<String, Decimal> = HashMap::new();

        for pos in self.positions.values() {
            let key = format!("{:?}", pos.asset.asset_class);
            let rate = fx
                .get(&pos.asset.base_currency)
                .copied()
                .unwrap_or(Decimal::ONE);
            *by_class.entry(key).or_default() += pos.notional_value * rate;
        }

        by_class.iter_mut().for_each(|(_, v)| *v /= nav);
        by_class
    }
}

#[derive(Debug)]
pub enum MarginEvent {
    MarginCall { symbol: String, deficit: Decimal },
    Liquidation { symbol: String, reason: String },
    PortfolioLeverageBreached { current: Decimal, max: Decimal },
}

pub struct MarginEngine {
    pub margin_call_buffer: Decimal,
}

impl MarginEngine {
    pub fn scan(
        &self,
        portfolio: &LeveragedPortfolio,
        fx: &HashMap<String, Decimal>,
    ) -> Vec<MarginEvent> {
        let mut events = vec![];
        for (sym, pos) in &portfolio.positions {
            if pos.current_price <= pos.liquidation_price && pos.direction == Direction::Long {
                events.push(MarginEvent::Liquidation {
                    symbol: sym.clone(),
                    reason: format!(
                        "Price {:.4} hit liquidation level {:.4}",
                        pos.current_price, pos.liquidation_price
                    ),
                });
            } else if pos.is_margin_call() {
                let required = pos.notional_value * pos.maintenance_margin_rate;
                let deficit = required - pos.current_equity();

                events.push(MarginEvent::MarginCall {
                    symbol: sym.clone(),
                    deficit,
                });
            }
        }
        // Portfolio-level leverage breach
        let gross_lev = portfolio.gross_leverage(fx);
        if gross_lev > portfolio.max_gross_leverage {
            events.push(MarginEvent::PortfolioLeverageBreached {
                current: gross_lev,
                max: portfolio.max_gross_leverage,
            });
        }

        events
    }
}
