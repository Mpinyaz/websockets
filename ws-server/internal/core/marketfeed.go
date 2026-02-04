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

// TiingoEquityData represents a single market update from IEX/Tiingo
type TiingoEquityData struct {
	UpdateType string    `json:"update_type"` // "T" = trade, "Q" = quote, "B" = break
	Date       time.Time `json:"date"`        // ISO timestamp from IEX
	Nanos      int64     `json:"nanoseconds"` // Nanoseconds since POSIX epoch
	Ticker     string    `json:"ticker"`

	// Quote fields (only for Q)
	BidSize  *int32   `json:"bid_size,omitempty"`
	BidPrice *float64 `json:"bid_price,omitempty"`
	MidPrice *float64 `json:"mid_price,omitempty"`
	AskPrice *float64 `json:"ask_price,omitempty"`
	AskSize  *int32   `json:"ask_size,omitempty"`

	// Trade / Break fields (only for T / B)
	LastPrice *float64 `json:"last_price,omitempty"`
	LastSize  *int32   `json:"last_size,omitempty"`

	// Trading state
	Halted     int32  `json:"halted"`                 // 1 if halted, 0 otherwise
	AfterHours int32  `json:"after_hours"`            // 1 if after hours
	ISO        int32  `json:"iso"`                    // Intermarket Sweep Order
	Oddlot     *int32 `json:"oddlot,omitempty"`       // only for trade
	NMSRule611 *int32 `json:"nms_rule_611,omitempty"` // only for trade
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
