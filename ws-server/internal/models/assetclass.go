package models

type AssetClass int

const (
	Forex AssetClass = iota
	Equity
	Crypto
)

var assetName = map[AssetClass]string{
	Forex:  "fx",
	Equity: "iex",
	Crypto: "crypto_data",
}

func (as AssetClass) String() string {
	return assetName[as]
}
