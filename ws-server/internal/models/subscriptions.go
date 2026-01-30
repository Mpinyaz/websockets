package models

import (
	"github.com/bwmarrin/snowflake"
)

type ClientSub struct {
	ID     snowflake.ID `json:"id,omitempty"`
	Forex  *[]string    `json:"forex,omitempty"`
	Equity *[]string    `json:"equity,omitempty"`
	Crypto *[]string    `json:"crypto,omitempty"`
}
