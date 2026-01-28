use core::fmt;

#[derive(Clone, Copy)]
pub enum AssetClass {
    Crypto,
    Forex,
    Equity,
}

impl fmt::Display for AssetClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetClass::Crypto => write!(f, "crypto"),
            AssetClass::Forex => write!(f, "fx"),
            AssetClass::Equity => write!(f, "iex"),
        }
    }
}
impl fmt::Debug for AssetClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}
