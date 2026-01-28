package core

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

// TiingoSubscribeRequest for subscribing to Tiingo forex feed
type TiingoSubscribeRequest struct {
	EventName     string              `json:"eventName"`
	Authorization string              `json:"authorization"`
	EventData     TiingoSubscribeData `json:"eventData"`
}

// TiingoSubscribeData for eventData field
type TiingoSubscribeData struct {
	SubscriptionID *string  `json:"subscriptionId,omitempty"` // Optional, for modifying existing subscriptions
	ThresholdLevel *string  `json:"thresholdLevel,omitempty"` // Optional threshold level as STRING (e.g., "7")
	Tickers        []string `json:"tickers,omitempty"`        // Optional list of tickers
}

// TiingoResponse from Tiingo WebSocket
type TiingoResponse struct {
	MessageType string      `json:"messageType"`       // "I" = Info, "H" = Heartbeat, "A" = Data
	Service     string      `json:"service,omitempty"` // "fx" for forex data
	Data        interface{} `json:"data,omitempty"`    // Can be object or array depending on messageType
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

// TiingoClient manages the connection to Tiingo
type TiingoClient struct {
	conn          *websocket.Conn
	apiKey        string
	manager       *Manager
	subscriptions map[string]bool
	mu            sync.RWMutex
	done          chan struct{}
}

var tiingoClient *TiingoClient

// InitTiingoClient creates and connects to Tiingo WebSocket
func InitTiingoClient(apiKey string, manager *Manager) error {
	if apiKey == "" {
		return fmt.Errorf("tiingo API key is required")
	}

	tiingoClient = &TiingoClient{
		apiKey:        apiKey,
		manager:       manager,
		subscriptions: make(map[string]bool),
		done:          make(chan struct{}),
	}

	return tiingoClient.Connect()
}

// Connect establishes WebSocket connection to Tiingo
func (tc *TiingoClient) Connect() error {
	dialer := websocket.Dialer{
		HandshakeTimeout: 10 * time.Second,
	}

	tiingoWSURL := os.Getenv("TIINGO_WS_URL")
	if tiingoWSURL == "" {
		tiingoWSURL = "wss://api.tiingo.com/fx"
	}

	log.Printf("Attempting to connect to Tiingo at: %s", tiingoWSURL)

	conn, resp, err := dialer.Dial(tiingoWSURL, nil)
	if err != nil {
		if resp != nil {
			log.Printf("Tiingo handshake failed - Status: %d %s", resp.StatusCode, resp.Status)
			// Read response body for more details
			if resp.Body != nil {
				body := make([]byte, 1024)
				n, _ := resp.Body.Read(body)
				if n > 0 {
					log.Printf("Response body: %s", string(body[:n]))
				}
			}
		}
		return fmt.Errorf("failed to connect to Tiingo: %w", err)
	}

	tc.conn = conn
	log.Println("✅ Connected to Tiingo Forex WebSocket")

	go tc.readMessages()

	return nil
}

// readMessages reads data from Tiingo and broadcasts to clients
func (tc *TiingoClient) readMessages() {
	defer func() {
		tc.conn.Close()
		close(tc.done)
	}()

	for {
		var response TiingoResponse
		err := tc.conn.ReadJSON(&response)
		if err != nil {
			log.Printf("Tiingo read error: %v", err)
			return
		}

		switch response.MessageType {
		case "I": // Info/Subscription response
			if response.Response != nil {
				log.Printf("Tiingo Info - Code: %d, Message: %s", response.Response.Code, response.Response.Message)
			}
			if response.Data != nil {
				log.Printf("Tiingo subscription data: %v", response.Data)
			}

		case "H": // Heartbeat
			log.Println("Tiingo heartbeat received")

		case "A": // Data update
			tc.handleDataUpdate(response)

		case "E": // Error message
			if response.Response != nil {
				log.Printf("❌ Tiingo ERROR - Code: %d, Message: %s", response.Response.Code, response.Response.Message)
			}
			if response.Data != nil {
				log.Printf("Error details: %v", response.Data)
			}

		default:
			log.Printf("Unknown Tiingo message type: %s, Full response: %+v", response.MessageType, response)
		}
	}
}

// handleDataUpdate processes forex data from Tiingo
func (tc *TiingoClient) handleDataUpdate(response TiingoResponse) {
	row, ok := response.Data.([]interface{})
	if !ok || len(row) < 8 {
		log.Printf("Invalid FX payload: %T %v", response.Data, response.Data)
		return
	}

	forexData := TiingoForexData{
		Type:      getString(row[0]), // "Q"
		Ticker:    getString(row[1]),
		Timestamp: getString(row[2]),
		BidSize:   getFloat(row[3]),
		BidPrice:  getFloat(row[4]),
		MidPrice:  getFloat(row[5]),
		AskPrice:  getFloat(row[6]), // Index 6: Ask Price
		AskSize:   getFloat(row[7]), // Index 7: Ask Size
	}

	payloadBytes, _ := json.Marshal(forexData)

	event := &Event{
		Type:    "forex_update",
		Payload: json.RawMessage(payloadBytes),
		Time:    time.Now(),
	}

	// Non-blocking send to broadcast channel
	select {
	case tc.manager.Broadcast <- event:
	default:
		log.Printf("Warning: Broadcast channel full, dropping forex update for %s", forexData.Ticker)
	}

	log.Printf(
		"FX %s | Bid %.5f | Ask %.5f | Mid %.5f",
		forexData.Ticker,
		forexData.BidPrice,
		forexData.AskPrice,
		forexData.MidPrice,
	)
}

// Helper functions to safely convert interface{} values
func getString(val interface{}) string {
	if s, ok := val.(string); ok {
		return s
	}
	return ""
}

func getFloat(val interface{}) float64 {
	switch v := val.(type) {
	case float64:
		return v
	case int:
		return float64(v)
	case int64:
		return float64(v)
	default:
		return 0.0
	}
}

// Subscribe subscribes to specific forex pairs on Tiingo
func (tc *TiingoClient) Subscribe(tickers []string, thresholdLevel *string) error {
	tc.mu.Lock()
	defer tc.mu.Unlock()

	eventData := TiingoSubscribeData{
		Tickers: tickers,
	}

	// Only set thresholdLevel if provided
	if thresholdLevel != nil {
		eventData.ThresholdLevel = thresholdLevel
	}

	subscribe := TiingoSubscribeRequest{
		EventName:     "subscribe",
		Authorization: tc.apiKey,
		EventData:     eventData,
	}

	if err := tc.conn.WriteJSON(subscribe); err != nil {
		return fmt.Errorf("failed to subscribe: %v", err)
	}

	// Track subscriptions
	for _, ticker := range tickers {
		tc.subscriptions[ticker] = true
	}

	thresholdStr := "default"
	if thresholdLevel != nil {
		thresholdStr = *thresholdLevel
	}
	log.Printf("Subscribed to Tiingo tickers: %v (threshold: %s)", tickers, thresholdStr)
	return nil
}

// Unsubscribe from specific tickers
func (tc *TiingoClient) Unsubscribe(tickers []string, subscriptionID string) error {
	tc.mu.Lock()
	defer tc.mu.Unlock()

	unsubscribe := map[string]interface{}{
		"eventName":     "unsubscribe",
		"authorization": tc.apiKey,
		"eventData": map[string]interface{}{
			"subscriptionId": subscriptionID,
			"tickers":        tickers,
		},
	}

	if err := tc.conn.WriteJSON(unsubscribe); err != nil {
		return fmt.Errorf("failed to unsubscribe: %v", err)
	}

	// Remove from tracked subscriptions
	for _, ticker := range tickers {
		delete(tc.subscriptions, ticker)
	}

	log.Printf("Unsubscribed from Tiingo tickers: %v", tickers)
	return nil
}

// HandleForexSubscribe allows clients to request forex pair subscriptions
func HandleForexSubscribe(c *Client, event Event) error {
	var payload struct {
		Tickers        []string `json:"tickers"`
		ThresholdLevel *string  `json:"thresholdLevel"` // Optional, "7" = bid/ask price or size changes
	}

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return fmt.Errorf("invalid payload: %v", err)
	}

	// Validate tickers
	if len(payload.Tickers) == 0 {
		return fmt.Errorf("at least one ticker is required")
	}

	// Subscribe to Tiingo
	if tiingoClient == nil {
		return fmt.Errorf("Tiingo client not initialized")
	}

	err := tiingoClient.Subscribe(payload.Tickers, payload.ThresholdLevel)
	if err != nil {
		return err
	}

	// Send acknowledgment to client (non-blocking)
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status":         "subscribed",
		"tickers":        payload.Tickers,
		"thresholdLevel": payload.ThresholdLevel,
	})

	ackEvent := &Event{
		Type:    "forex_subscribed",
		Payload: json.RawMessage(ackPayload),
		Time:    time.Now(),
	}

	select {
	case c.Send <- ackEvent:
	default:
		log.Printf("Failed to send forex subscribe ack to client %s: channel full", c.ID)
	}

	return nil
}

// HandleForexUnsubscribe allows clients to unsubscribe from forex pairs
func HandleForexUnsubscribe(c *Client, event Event) error {
	var payload struct {
		Tickers        []string `json:"tickers"`
		SubscriptionID string   `json:"subscriptionId"`
	}

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return fmt.Errorf("invalid payload: %v", err)
	}

	if tiingoClient == nil {
		return fmt.Errorf("Tiingo client not initialized")
	}

	err := tiingoClient.Unsubscribe(payload.Tickers, payload.SubscriptionID)
	if err != nil {
		return err
	}

	// Send acknowledgment (non-blocking)
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status":  "unsubscribed",
		"tickers": payload.Tickers,
	})

	ackEvent := &Event{
		Type:    "forex_unsubscribed",
		Payload: json.RawMessage(ackPayload),
		Time:    time.Now(),
	}

	select {
	case c.Send <- ackEvent:
	default:
		log.Printf("Failed to send forex unsubscribe ack to client %s: channel full", c.ID)
	}

	return nil
}

// Close cleanly closes the Tiingo connection
func (tc *TiingoClient) Close() {
	if tc.conn != nil {
		tc.conn.Close()
	}
	<-tc.done
}

// GetTiingoClient returns the global Tiingo client instance
func GetTiingoClient() *TiingoClient {
	return tiingoClient
}
