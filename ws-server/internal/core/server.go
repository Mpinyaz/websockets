package core

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"sync"
	"time"

	"websockets/internal/models"
	"websockets/internal/store/cache"
	"websockets/utils"

	"github.com/bwmarrin/snowflake"
	"github.com/gorilla/websocket"
)

const (
	writeWait      = 10 * time.Second
	pongWait       = 60 * time.Second
	pingPeriod     = (pongWait * 9) / 10
	maxMessageSize = 8192
)

var upgrader = websocket.Upgrader{
	CheckOrigin:     func(r *http.Request) bool { return true },
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
}

type Event struct {
	Type    string          `json:"type"`
	Payload json.RawMessage `json:"payload"`
	From    snowflake.ID    `json:"from,omitempty"`
	Time    time.Time       `json:"time"`
}

type Manager struct {
	Clients       map[*Client]bool
	Mu            sync.RWMutex
	Broadcast     chan *Event
	Register      chan *Client
	Unregister    chan *Client
	Handlers      map[string]EventHandler
	configuration *utils.Config
	clientStore   *cache.ClientStore
	idNode        *snowflake.Node
	streams       *MdwsStreams
}

type Client struct {
	Mgmt *Manager
	Conn *websocket.Conn
	Send chan *Event
	ID   snowflake.ID
	Done chan struct{}
}

type EventHandler func(c *Client, event Event) error

func (m *Manager) CreateClient(conn *websocket.Conn) *Client {
	client := &Client{
		Mgmt: m,
		Conn: conn,
		Send: make(chan *Event, 256), // buffered to prevent blocking
		ID:   m.idNode.Generate(),
		Done: make(chan struct{}),
	}

	m.Register <- client

	go client.writePump()
	go client.readPump()

	return client
}

func NewManager(cfg *utils.Config, clientStore *cache.ClientStore) *Manager {
	node, _ := snowflake.NewNode(cfg.SfnodeID)

	m := &Manager{
		Clients:       make(map[*Client]bool),
		Broadcast:     make(chan *Event, 1000),
		Register:      make(chan *Client),
		Unregister:    make(chan *Client),
		Handlers:      make(map[string]EventHandler),
		configuration: cfg,
		clientStore:   clientStore,
		idNode:        node,
		streams:       nil,
	}
	m.setupEventHandlers()
	return m
}

func (m *Manager) SetStreams(streams *MdwsStreams) {
	m.streams = streams
}

func (m *Manager) setupEventHandlers() {
	m.Handlers["broadcast"] = HandleBroadcast
	m.Handlers["ping"] = HandlePing
	m.Handlers["subscribe"] = func(c *Client, e Event) error {
		if m.streams == nil {
			return fmt.Errorf("streams not initialized")
		}
		return m.HandleSubscribe(c, e, m.streams)
	}
	m.Handlers["unsubscribe"] = func(c *Client, e Event) error {
		if m.streams == nil {
			return fmt.Errorf("streams not initialized")
		}
		return m.HandleUnsubscribe(c, e, m.streams)
	}
	m.Handlers["patch_subscribe"] = func(c *Client, e Event) error {
		if m.streams == nil {
			return fmt.Errorf("streams not initialized")
		}
		return m.HandlePatchSubscribe(c, e, m.streams)
	}
}

func (m *Manager) RegisterHandler(eventType string, handler EventHandler) {
	m.Handlers[eventType] = handler
}

func (m *Manager) NumClients() int {
	m.Mu.RLock()
	defer m.Mu.RUnlock()
	return len(m.Clients)
}

func (m *Manager) removeClientNow(c *Client, reason string) {
	m.Mu.Lock()
	defer m.Mu.Unlock()

	if _, ok := m.Clients[c]; !ok {
		return // Already removed
	}

	delete(m.Clients, c)
	close(c.Done)
	close(c.Send)

	// Remove subscriptions from Redis if configured
	if m.clientStore != nil {
		if err := m.clientStore.RemoveClientSubs(c.ID.Int64()); err != nil {
			log.Printf("Failed to remove client %s from Redis: %v", c.ID, err)
		}
	}

	log.Printf("Removing client %s: %s", c.ID, reason)
}

func (m *Manager) Run() {
	for {
		select {
		case client := <-m.Register:
			m.Mu.Lock()
			m.Clients[client] = true
			m.Mu.Unlock()
			log.Printf("Client %s registered. Total clients: %d", client.ID, m.NumClients())

		case client := <-m.Unregister:
			m.removeClientNow(client, "unregistered")

		case message := <-m.Broadcast:
			m.Mu.RLock()
			for client := range m.Clients {
				select {
				case client.Send <- message:
					// Sent successfully
				default:
					// Slow client, remove immediately
					go m.removeClientNow(client, "slow consumption")
				}
			}
			m.Mu.RUnlock()
		}
	}
}

// HandleSubscribe handles adding to existing subscriptions
func (m *Manager) HandleSubscribe(c *Client, event Event, conn *MdwsStreams) error {
	var payload TiingoSubscribeData

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return fmt.Errorf("invalid payload: %w", err)
	}

	if len(payload.Tickers) == 0 || payload.AssetClass == "" {
		return fmt.Errorf("asset and symbol is required")
	}

	var asset models.AssetClass
	switch payload.AssetClass {
	case "forex":
		asset = models.Forex
	case "equity":
		asset = models.Equity
	case "crypto":
		asset = models.Crypto
	default:
		return fmt.Errorf("invalid asset type: %s", payload.AssetClass)
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

	// Create subscription object
	sub := models.ClientSub{
		ID: c.ID,
	}

	// Set the appropriate field
	switch asset {
	case models.Forex:
		sub.Forex = &payload.Tickers
	case models.Equity:
		sub.Equity = &payload.Tickers
	case models.Crypto:
		sub.Crypto = &payload.Tickers
	}

	// Patch in Redis (adds to existing)
	if m.clientStore != nil {
		if err := m.clientStore.PatchClientSub(asset, sub); err != nil {
			return err
		}
	}

	// Send acknowledgment
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status":  "subscribed",
		"asset":   payload.AssetClass,
		"symbols": payload.Tickers,
	})

	ackEvent := &Event{
		Type:    "subscribe",
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

// HandlePatchSubscribe handles adding to existing subscriptions
func (m *Manager) HandlePatchSubscribe(c *Client, event Event, conn *MdwsStreams) error {
	var payload TiingoSubscribeData

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return fmt.Errorf("invalid payload: %w", err)
	}

	if len(payload.Tickers) == 0 || payload.AssetClass == "" {
		return fmt.Errorf("asset and symbol is required")
	}

	var asset models.AssetClass
	switch payload.AssetClass {
	case "forex":
		asset = models.Forex
	case "equity":
		asset = models.Equity
	case "crypto":
		asset = models.Crypto
	default:
		return fmt.Errorf("invalid asset type: %s", payload.AssetClass)
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

	// Create subscription object
	sub := models.ClientSub{
		ID: c.ID,
	}

	// Set the appropriate field
	switch asset {
	case models.Forex:
		sub.Forex = &payload.Tickers
	case models.Equity:
		sub.Equity = &payload.Tickers
	case models.Crypto:
		sub.Crypto = &payload.Tickers
	}

	// Patch in Redis (adds to existing)
	if m.clientStore != nil {
		if err := m.clientStore.PatchClientSub(asset, sub); err != nil {
			return err
		}
	}

	// Send acknowledgment
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status":  "patched",
		"asset":   payload.AssetClass,
		"symbols": payload.Tickers,
	})

	ackEvent := &Event{
		Type:    "patch_subscribe",
		Payload: json.RawMessage(ackPayload),
		Time:    time.Now(),
		From:    c.ID,
	}

	select {
	case c.Send <- ackEvent:
	default:
		log.Printf("Failed to send patch subscribe ack to client %s", c.ID)
	}

	return nil
}

// HandleUnsubscribe handles unsubscribing from symbols

func (m *Manager) HandleUnsubscribe(
	c *Client,
	event Event,
	conn *MdwsStreams,
) error {
	var payload TiingoSubscribeData

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return fmt.Errorf("invalid payload: %w", err)
	}

	if payload.AssetClass == "" {
		return fmt.Errorf("assetClass is required")
	}

	var asset models.AssetClass
	switch payload.AssetClass {
	case "forex":
		asset = models.Forex
	case "equity":
		asset = models.Equity
	case "crypto":
		asset = models.Crypto
	default:
		return fmt.Errorf("invalid asset type: %s", payload.AssetClass)
	}

	// ------------------------------------------------------------
	// Publish unsubscribe to RabbitMQ
	// ------------------------------------------------------------
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

	// ------------------------------------------------------------
	// Update Redis (remove subscriptions)
	// ------------------------------------------------------------
	if m.clientStore != nil {
		// Currently remove ALL symbols for that asset
		if err := m.clientStore.RemoveClientSubsByAsset(
			c.ID.Int64(),
			asset,
		); err != nil {
			return err
		}
	}

	// ------------------------------------------------------------
	// ACK client
	// ------------------------------------------------------------
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status": "unsubscribed",
		"asset":  payload.AssetClass,
	})

	ackEvent := &Event{
		Type:    "unsubscribed",
		Payload: json.RawMessage(ackPayload),
		Time:    time.Now(),
		From:    c.ID,
	}

	select {
	case c.Send <- ackEvent:
	default:
		log.Printf("Failed to send unsubscribe ack to client %s", c.ID)
	}

	return nil
}

func (c *Client) readPump() {
	defer func() {
		// Unregister the client from the manager
		c.Mgmt.Unregister <- c
		// Close the websocket connection
		c.Conn.Close()
	}()

	c.Conn.SetReadLimit(maxMessageSize)
	c.Conn.SetReadDeadline(time.Now().Add(pongWait))
	c.Conn.SetPongHandler(func(appData string) error {
		// Reset read deadline on pong
		c.Conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	for {
		var event Event
		err := c.Conn.ReadJSON(&event)
		if err != nil {
			// Only log unexpected close errors
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("Read error from client %s: %v", c.ID, err)
			} else {
				log.Printf("Client %s disconnected: %v", c.ID, err)
			}
			break
		}

		// Attach sender and timestamp
		event.From = c.ID
		event.Time = time.Now()
		log.Printf("Received from %s: type=%s, payload=%s", c.ID, event.Type, string(event.Payload))

		// Handle the event if a handler exists
		if handler, ok := c.Mgmt.Handlers[event.Type]; ok {
			if err := handler(c, event); err != nil {
				log.Printf("Handler error for event type '%s': %v", event.Type, err)
				c.sendError(err.Error())
			}
		} else {
			// Unknown event type
			log.Printf("Unknown event from %s: type=%s", c.ID, event.Type)
			c.sendUnknownEvent(event.Type)
		}
	}
}

func (c *Client) writePump() {
	ticker := time.NewTicker(pingPeriod)
	defer func() {
		ticker.Stop()
		c.Conn.Close()
	}()

	for {
		select {
		case <-c.Done:
			return

		case message, ok := <-c.Send:
			c.Conn.SetWriteDeadline(time.Now().Add(writeWait))
			if !ok {
				// Channel closed; send close message
				_ = c.Conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			if err := c.Conn.WriteJSON(message); err != nil {
				log.Printf("Write error to client %s: %v", c.ID, err)
				return
			}

		case <-ticker.C:
			// Send ping periodically to keep connection alive
			c.Conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := c.Conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				log.Printf("Ping error to client %s: %v", c.ID, err)
				return
			}
		}
	}
}

// Helper methods
func (c *Client) sendError(errMsg string) {
	errorPayload, _ := json.Marshal(map[string]string{"message": errMsg})
	errorEvent := &Event{
		Type:    "error",
		Payload: json.RawMessage(errorPayload),
		Time:    time.Now(),
	}

	select {
	case c.Send <- errorEvent:
	default:
		log.Printf("Failed to send error to client %s: channel full", c.ID)
	}
}

func (c *Client) sendUnknownEvent(eventType string) {
	unknownPayload, _ := json.Marshal(map[string]string{
		"message": "Unknown event type received",
		"type":    eventType,
	})
	unknownEvent := &Event{
		Type:    "unknown_event",
		Payload: json.RawMessage(unknownPayload),
		Time:    time.Now(),
	}

	select {
	case c.Send <- unknownEvent:
	default:
		log.Printf("Failed to send unknown event response to client %s: channel full", c.ID)
	}
}

// GetConfig returns the manager's configuration
func (m *Manager) GetConfig() *utils.Config {
	return m.configuration
}
