package core

import (
	"encoding/json"
	"fmt"
	"log"
	"time"
)

// TiingoSubscribeData for eventData field
type TiingoSubscribeData struct {
	AssetClass     string   `json:"assetClass"`
	ThresholdLevel *string  `json:"thresholdLevel,omitempty"` // Optional threshold level as STRING (e.g., "7")
	Tickers        []string `json:"tickers,omitempty"`        // Optional list of tickers
}

// TiingoResponse from Tiingo WebSocket
type TiingoResponse struct {
	MessageType string          `json:"messageType"`       // "I" = Info, "H" = Heartbeat, "A" = Data
	Service     string          `json:"service,omitempty"` // "fx" for forex data
	Data        json.RawMessage `json:"data,omitempty"`    // Can be object or array depending on messageType
	Response    *struct {
		Code    int    `json:"code"`
		Message string `json:"message"`
	} `json:"response,omitempty"`
}

// TiingoForexData parsed forex quote data
// Array format: [type, ticker, timestamp, bidSize, bidPrice, midPrice, askPrice, askSize]
type TiingoForexData struct {
	Type      string  `json:"type"`      // Index 0: "Q" for quote update
	Ticker    string  `json:"ticker"`    // Index 1: e.g., "eurusd"
	Timestamp string  `json:"timestamp"` // Index 2: ISO datetime
	BidSize   float64 `json:"bidSize"`   // Index 3: Number of units at bid
	BidPrice  float64 `json:"bidPrice"`  // Index 4: Current highest bid price
	MidPrice  float64 `json:"midPrice"`  // Index 5: (bidPrice + askPrice) / 2
	AskPrice  float64 `json:"askPrice"`  // Index 6: Current lowest ask price
	AskSize   float64 `json:"askSize"`   // Index 7: Number of units at ask
}

// EquityUpdate represents a single market update for equities, primarily from Alpaca,
// with descriptive camelCase field names for frontend compatibility.
type EquityUpdate struct {
	UpdateType string    `json:"updateType"` // "T" = trade, "Q" = quote, "B" = bar
	Date       time.Time `json:"date"`       // RFC3339 timestamp
	Nanos      int64     `json:"nanoseconds"`
	Ticker     string    `json:"ticker"`

	// Quote fields
	BidSize     *float64 `json:"bidSize,omitempty"`
	BidPrice    *float64 `json:"bidPrice,omitempty"`
	MidPrice    *float64 `json:"midPrice,omitempty"`
	AskPrice    *float64 `json:"askPrice,omitempty"`
	AskSize     *float64 `json:"askSize,omitempty"`
	BidExchange string   `json:"bidExchange,omitempty"`
	AskExchange string   `json:"askExchange,omitempty"`

	// Trade fields
	LastPrice *float64 `json:"lastPrice,omitempty"`
	LastSize  *float64 `json:"lastSize,omitempty"`
	TradeID   *uint64  `json:"tradeId,omitempty"`
	Exchange  string   `json:"exchange,omitempty"` // Original 'x' from Alpaca

	// Bar fields
	Open       *float64 `json:"open,omitempty"`
	High       *float64 `json:"high,omitempty"`
	Low        *float64 `json:"low,omitempty"`
	Close      *float64 `json:"close,omitempty"`
	Volume     *float64 `json:"volume,omitempty"`
	TradeCount *uint64  `json:"tradeCount,omitempty"`
	VWAP       *float64 `json:"vwap,omitempty"`

	// Common
	Conditions []string `json:"conditions,omitempty"` // Original 'c' from Alpaca
	Tape       string   `json:"tape,omitempty"`       // Original 'z' from Alpaca
}

// Alpaca-specific structs to aid in initial unmarshaling
type AlpacaTrade struct {
	Symbol     string   `json:"S"`
	Price      float64  `json:"p"`
	Size       float64  `json:"s"`
	Timestamp  string   `json:"t"`
	TradeID    uint64   `json:"i"`
	Exchange   string   `json:"x"`
	Tape       string   `json:"z"`
	Conditions []string `json:"c"`
}

type AlpacaQuote struct {
	Symbol      string   `json:"S"`
	BidPrice    float64  `json:"bp"`
	BidSize     float64  `json:"bs"`
	AskPrice    float64  `json:"ap"`
	AskSize     float64  `json:"as"`
	Timestamp   string   `json:"t"`
	BidExchange string   `json:"bx"`
	AskExchange string   `json:"ax"`
	Conditions  []string `json:"c"`
	Tape        string   `json:"z"`
}

type AlpacaBar struct {
	Symbol     string  `json:"S"`
	Open       float64 `json:"o"`
	High       float64 `json:"h"`
	Low        float64 `json:"l"`
	Close      float64 `json:"c"`
	Volume     float64 `json:"v"`
	Timestamp  string  `json:"t"`
	TradeCount uint64  `json:"n"`
	VWAP       float64 `json:"vw"`
}

type TiingoCryptoData struct {
	UpdateType string    `json:"update_type"` // "T" = last trade
	Ticker     string    `json:"ticker"`      // Index 1
	Date       time.Time `json:"date"`        // Index 2
	Exchange   string    `json:"exchange"`    // Index 3
	LastSize   float64   `json:"last_size"`   // Index 4
	LastPrice  float64   `json:"last_price"`  // Index 5
}

// HandleSubscribe allows clients to request subscriptions
func HandleSubscribe(c *Client, event Event, conn *MdwsStreams) error {
	var payload TiingoSubscribeData

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return fmt.Errorf("invalid payload: %w", err)
	}

	if len(payload.Tickers) == 0 {
		return fmt.Errorf("at least one ticker is required")
	}
	internalEvent := &Event{
		Type:    "subscribe",
		Payload: event.Payload,
		Time:    time.Now(),
		From:    c.ID,
	}

	if err := conn.PublishSubscription(internalEvent); err != nil {
		log.Printf("Failed to publish subscription: %v", err)
		return err
	}
	// Acknowledge client
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status":         "subscribed",
		"assetClass":     payload.AssetClass,
		"tickers":        payload.Tickers,
		"thresholdLevel": payload.ThresholdLevel,
	})

	ackEvent := &Event{
		Type:    "subscribed",
		Payload: json.RawMessage(ackPayload),
		Time:    time.Now(),
		From:    c.ID,
	}

	select {
	case c.Send <- ackEvent:
	default:
		log.Printf("Failed to send subscribe ack to client %s", c.ID)
	}

	return nil
}

// HandleUnsubscribe allows clients to request subscriptions
func HandleUnsubscribe(c *Client, event Event, conn *MdwsStreams) error {
	var payload TiingoSubscribeData

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return fmt.Errorf("invalid payload: %w", err)
	}

	if len(payload.Tickers) == 0 {
		return fmt.Errorf("at least one ticker is required")
	}
	internalEvent := &Event{
		Type:    "unsubscribe",
		Payload: event.Payload,
		Time:    time.Now(),
		From:    c.ID,
	}

	if err := conn.PublishSubscription(internalEvent); err != nil {
		log.Printf("Failed to publish unsubscribe: %v", err)
		return err
	}
	// Acknowledge client
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status":         "subscribed",
		"assetClass":     payload.AssetClass,
		"tickers":        payload.Tickers,
		"thresholdLevel": payload.ThresholdLevel,
	})

	ackEvent := &Event{
		Type:    "subscribed",
		Payload: json.RawMessage(ackPayload),
		Time:    time.Now(),
		From:    c.ID,
	}

	select {
	case c.Send <- ackEvent:
	default:
		log.Printf("Failed to send subscribe ack to client %s", c.ID)
	}

	return nil
}
