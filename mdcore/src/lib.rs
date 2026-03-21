use core::fmt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Hash, Copy)]
#[serde(rename_all = "camelCase")]
pub enum AssetClass {
    Crypto,
    Forex,
    Equity,
}

impl fmt::Debug for AssetClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl AssetClass {
    pub fn measurement(&self) -> &'static str {
        match self {
            AssetClass::Crypto => "crypto",
            AssetClass::Forex => "forex",
            AssetClass::Equity => "equity",
        }
    }
}

impl fmt::Display for AssetClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetClass::Crypto => write!(f, "crypto"),
            AssetClass::Forex => write!(f, "forex"),
            AssetClass::Equity => write!(f, "equity"),
        }
    }
}

impl FromStr for AssetClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "crypto" | "crypto_data" => Ok(AssetClass::Crypto),
            "forex" => Ok(AssetClass::Forex),
            "equity" => Ok(AssetClass::Equity),
            _ => Err(format!("Unknown AssetClass: {}", s)),
        }
    }
}
