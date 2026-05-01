use crate::{data::pricing::MarketSnapshot, Asset};
use mdcore::AssetClass;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

use crate::strategy::AllocationStrategy;

pub struct EqualWeightStrategy {
    pub equity_alloc: Decimal,
    pub forex_alloc: Decimal,
    pub crypto_alloc: Decimal,
}

impl AllocationStrategy for EqualWeightStrategy {
    fn name(&self) -> &str {
        "equal_weight"
    }

    fn compute_weights(
        &self,
        assets: &[Asset],
        _snapshot: &MarketSnapshot,
    ) -> HashMap<String, Decimal> {
        let equities: Vec<_> = assets
            .iter()
            .filter(|a| a.asset_class == AssetClass::Equity)
            .collect();
        let cryptos: Vec<_> = assets
            .iter()
            .filter(|a| a.asset_class == AssetClass::Crypto)
            .collect();
        let forex: Vec<_> = assets
            .iter()
            .filter(|a| a.asset_class == AssetClass::Forex)
            .collect();

        let mut w = HashMap::new();
        for a in &equities {
            w.insert(
                a.symbol.clone(),
                self.equity_alloc / Decimal::from(equities.len()),
            );
        }
        for a in &cryptos {
            w.insert(
                a.symbol.clone(),
                self.crypto_alloc / Decimal::from(cryptos.len()),
            );
        }
        for a in &forex {
            w.insert(
                a.symbol.clone(),
                self.forex_alloc / Decimal::from(forex.len()),
            );
        }
        w
    }
}

pub struct RiskParityStrategy {
    pub target_volatility: Decimal,
}

impl AllocationStrategy for RiskParityStrategy {
    fn compute_weights(
        &self,
        assets: &[Asset],
        snapshot: &MarketSnapshot,
    ) -> HashMap<String, Decimal> {
        let inv_vols: Vec<(String, Decimal)> = assets
            .iter()
            .filter_map(|a| {
                let vol = snapshot.volatility.get(&a.symbol)?;
                if *vol == Decimal::ZERO {
                    return None;
                }
                Some((a.symbol.clone(), dec!(1) / vol))
            })
            .collect();

        let total_inv_vol: Decimal = inv_vols.iter().map(|(_, v)| v).sum();
        if total_inv_vol == Decimal::ZERO {
            return HashMap::new();
        }

        inv_vols
            .into_iter()
            .map(|(sym, inv_v)| (sym, inv_v / total_inv_vol))
            .collect()
    }

    fn name(&self) -> &str {
        "risk_parity"
    }
}
