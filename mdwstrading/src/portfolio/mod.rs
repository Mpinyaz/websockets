pub mod leveraged;
use crate::Position;
use rust_decimal::Decimal;
use std::collections::HashMap;
pub struct Portfolio {
    pub cash: HashMap<String, Decimal>,
    pub positions: HashMap<String, Position>,
    pub base_currency: String,
}

impl Portfolio {
    pub fn new(base_currency: &str, initial_cash: Decimal) -> Self {
        let mut cash = HashMap::new();
        cash.insert(base_currency.to_string(), initial_cash);
        Portfolio {
            cash,
            positions: HashMap::new(),
            base_currency: base_currency.to_string(),
        }
    }

    pub fn total_equity(&self, fx_rates: &HashMap<String, Decimal>) -> Decimal {
        let cash_value: Decimal = self
            .cash
            .iter()
            .map(|(ccy, amt)| {
                let rate = fx_rates.get(ccy).copied().unwrap_or(Decimal::ONE);
                amt * rate
            })
            .sum();

        let position_value: Decimal = self
            .positions
            .values()
            .map(|p| {
                let rate = fx_rates
                    .get(&p.asset.base_currency)
                    .copied()
                    .unwrap_or(Decimal::ONE);
                p.market_value() * rate
            })
            .sum();

        cash_value + position_value
    }

    pub fn weights(&self, fx_rates: &HashMap<String, Decimal>) -> HashMap<String, Decimal> {
        let total = self.total_equity(fx_rates);
        self.positions
            .iter()
            .map(|(sym, pos)| {
                let rate = fx_rates
                    .get(&pos.asset.base_currency)
                    .copied()
                    .unwrap_or(Decimal::ONE);
                let weight = (pos.market_value() * rate) / total;
                (sym.clone(), weight)
            })
            .collect()
    }
}
