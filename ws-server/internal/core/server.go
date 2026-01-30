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
}

type Client struct {
	Mgmt *Manager
	Conn *websocket.Conn
	Send chan *Event
	ID   snowflake.ID
	Done chan struct{}
}

type EventHandler func(c *Client, event Event) error

func NewManager(cfg *utils.Config, clientStore *cache.ClientStore) *Manager {
	node, _ := snowflake.NewNode(cfg.SfnodeID)

	m := &Manager{
		Clients:       make(map[*Client]bool),
		Broadcast:     make(chan *Event, 256),
		Register:      make(chan *Client),
		Unregister:    make(chan *Client),
		Handlers:      make(map[string]EventHandler),
		configuration: cfg,
		clientStore:   clientStore,
		idNode:        node,
	}
	m.setupEventHandlers()
	return m
}

func (m *Manager) setupEventHandlers() {
	m.Handlers["broadcast"] = HandleBroadcast
	m.Handlers["ping"] = HandlePing
	m.Handlers["subscribe"] = m.HandleSubscribe
	m.Handlers["unsubscribe"] = m.HandleUnsubscribe
	m.Handlers["patch_subscribe"] = m.HandlePatchSubscribe
	m.Handlers["forex_subscribe"] = HandleForexSubscribe
	m.Handlers["forex_unsubscribe"] = HandleForexUnsubscribe
}

func (m *Manager) RegisterHandler(eventType string, handler EventHandler) {
	m.Handlers[eventType] = handler
}

func (m *Manager) NumClients() int {
	m.Mu.RLock()
	defer m.Mu.RUnlock()
	return len(m.Clients)
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
			m.Mu.Lock()
			if _, ok := m.Clients[client]; ok {
				delete(m.Clients, client)
				close(client.Done)
				close(client.Send)

				// Remove client subscriptions from Redis
				if m.clientStore != nil {
					if err := m.clientStore.RemoveClientSubs(client.ID.Int64()); err != nil {
						log.Printf("Failed to remove client %s from Redis: %v", client.ID, err)
					}
				}

				log.Printf("Client %s unregistered. Total clients: %d", client.ID, m.NumClients())
			}
			m.Mu.Unlock()

		case message := <-m.Broadcast:
			m.Mu.RLock()
			clientsToRemove := []*Client{}

			for client := range m.Clients {
				select {
				case client.Send <- message:
					// Successfully sent
				default:
					// Channel full - mark for removal
					clientsToRemove = append(clientsToRemove, client)
					log.Printf("Client %s channel full, marking for removal", client.ID)
				}
			}
			m.Mu.RUnlock()

			// Remove slow clients outside of read lock
			if len(clientsToRemove) > 0 {
				m.Mu.Lock()
				for _, client := range clientsToRemove {
					if _, ok := m.Clients[client]; ok {
						delete(m.Clients, client)
						close(client.Done)
						close(client.Send)

						if m.clientStore != nil {
							if err := m.clientStore.RemoveClientSubs(client.ID.Int64()); err != nil {
								log.Printf("Failed to remove slow client %s from Redis: %v", client.ID, err)
							}
						}

						log.Printf("Client %s removed due to slow consumption", client.ID)
					}
				}
				m.Mu.Unlock()
			}
		}
	}
}

// HandleSubscribe handles full subscription replacement
func (m *Manager) HandleSubscribe(c *Client, event Event) error {
	var payload struct {
		Forex  *[]string `json:"forex,omitempty"`
		Equity *[]string `json:"equity,omitempty"`
		Crypto *[]string `json:"crypto,omitempty"`
	}

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return err
	}

	// Create subscription object
	sub := models.ClientSub{
		ID:     c.ID,
		Forex:  payload.Forex,
		Equity: payload.Equity,
		Crypto: payload.Crypto,
	}

	// Store in Redis (replaces existing subscriptions)
	if m.clientStore != nil {
		if err := m.clientStore.SetClientSubs(sub); err != nil {
			return err
		}
	}

	// Send acknowledgment
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status": "subscribed",
		"forex":  payload.Forex,
		"equity": payload.Equity,
		"crypto": payload.Crypto,
	})

	ackEvent := &Event{
		Type:    "subscribed",
		Payload: json.RawMessage(ackPayload),
		Time:    time.Now(),
	}

	select {
	case c.Send <- ackEvent:
	default:
		log.Printf("Failed to send subscribe ack to client %s", c.ID)
	}

	return nil
}

// HandlePatchSubscribe handles adding to existing subscriptions
func (m *Manager) HandlePatchSubscribe(c *Client, event Event) error {
	var payload struct {
		Asset   string   `json:"asset"` // "forex", "equity", or "crypto"
		Symbols []string `json:"symbols"`
	}

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return err
	}

	// Determine asset class
	var asset models.AssetClass
	switch payload.Asset {
	case "forex":
		asset = models.Forex
	case "equity":
		asset = models.Equity
	case "crypto":
		asset = models.Crypto
	default:
		return fmt.Errorf("invalid asset type: %s", payload.Asset)
	}

	// Create subscription object
	sub := models.ClientSub{
		ID: c.ID,
	}

	// Set the appropriate field
	switch asset {
	case models.Forex:
		sub.Forex = &payload.Symbols
	case models.Equity:
		sub.Equity = &payload.Symbols
	case models.Crypto:
		sub.Crypto = &payload.Symbols
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
		"asset":   payload.Asset,
		"symbols": payload.Symbols,
	})

	ackEvent := &Event{
		Type:    "patch_subscribed",
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
func (m *Manager) HandleUnsubscribe(c *Client, event Event) error {
	var payload struct {
		Asset *string `json:"asset,omitempty"` // Optional: specific asset class
	}

	if err := json.Unmarshal(event.Payload, &payload); err != nil {
		return err
	}

	var err error
	if payload.Asset != nil {
		// Unsubscribe from specific asset class
		var asset models.AssetClass
		switch *payload.Asset {
		case "forex":
			asset = models.Forex
		case "equity":
			asset = models.Equity
		case "crypto":
			asset = models.Crypto
		default:
			return fmt.Errorf("invalid asset type: %s", *payload.Asset)
		}

		if m.clientStore != nil {
			err = m.clientStore.RemoveClientSubsByAsset(c.ID.Int64(), asset)
		}
	} else {
		// Unsubscribe from all
		if m.clientStore != nil {
			err = m.clientStore.RemoveClientSubs(c.ID.Int64())
		}
	}

	if err != nil {
		return err
	}

	// Send acknowledgment
	ackPayload, _ := json.Marshal(map[string]interface{}{
		"status": "unsubscribed",
		"asset":  payload.Asset,
	})

	ackEvent := &Event{
		Type:    "unsubscribed",
		Payload: json.RawMessage(ackPayload),
		Time:    time.Now(),
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
		c.Mgmt.Unregister <- c
		c.Conn.Close()
	}()

	c.Conn.SetReadLimit(maxMessageSize)
	c.Conn.SetReadDeadline(time.Now().Add(pongWait))
	c.Conn.SetPongHandler(func(string) error {
		c.Conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	for {
		var event Event
		err := c.Conn.ReadJSON(&event)
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("Read error from client %s: %v", c.ID, err)
			}
			break
		}

		event.From = c.ID
		event.Time = time.Now()
		log.Printf("Received from %s: type=%s, payload=%s", c.ID, event.Type, string(event.Payload))

		if handler, ok := c.Mgmt.Handlers[event.Type]; ok {
			if err := handler(c, event); err != nil {
				log.Printf("Handler error for event type '%s': %v", event.Type, err)
				c.sendError(err.Error())
			}
		} else {
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
				c.Conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			w, err := c.Conn.NextWriter(websocket.TextMessage)
			if err != nil {
				log.Printf("Write error to client %s: %v", c.ID, err)
				return
			}
			json.NewEncoder(w).Encode(message)
			w.Close()

		case <-ticker.C:
			c.Conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := c.Conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				log.Println("ping error:", err)
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
