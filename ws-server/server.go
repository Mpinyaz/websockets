package main

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

const (
	writeWait      = 10 * time.Second
	pongWait       = 60 * time.Second
	pingPeriod     = (pongWait * 9) / 10
	maxMessageSize = 8192 // Increased for JSON payloads
)

var upgrader = websocket.Upgrader{
	CheckOrigin:     func(r *http.Request) bool { return true },
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
}

type Event struct {
	Type    string          `json:"type"`
	Payload json.RawMessage `json:"payload"`
	From    uuid.UUID       `json:"from,omitempty"`
	Time    time.Time       `json:"time"`
}

type Manager struct {
	clients    map[*Client]bool
	broadcast  chan *Event
	register   chan *Client
	unregister chan *Client
	handlers   map[string]EventHandler
}

type Client struct {
	mgmt *Manager
	conn *websocket.Conn
	send chan *Event
	id   uuid.UUID
}

// writePump handles outgoing messages to the websocket connection
func (c *Client) writePump() {
	ticker := time.NewTicker(pingPeriod)
	defer func() {
		ticker.Stop()
		c.conn.Close()
	}()

	for {
		select {
		case message, ok := <-c.send:
			c.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if !ok {
				// Hub closed the channel
				c.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			if err := c.conn.WriteJSON(message); err != nil {
				log.Printf("Write error to client %s: %v", c.id, err)
				return
			}

		case <-ticker.C:
			c.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}

func NewManager() *Manager {
	return &Manager{
		clients:    make(map[*Client]bool),
		broadcast:  make(chan *Event),
		register:   make(chan *Client),
		unregister: make(chan *Client),
		handlers:   make(map[string]EventHandler),
	}
}

func (m *Manager) setupEventHandlers() {
	m.handlers["broadcast"] = HandleBroadcast
	m.handlers["ping"] = HandlePing
}

func (m *Manager) RegisterHandler(eventType string, handler EventHandler) {
	m.handlers[eventType] = handler
}

func (m *Manager) Run() {
	for {
		select {
		case client := <-m.register:
			m.clients[client] = true
			log.Printf("Client %s registered. Total clients: %d", client.id, len(m.clients))

		case client := <-m.unregister:
			if _, ok := m.clients[client]; ok {
				delete(m.clients, client)
				close(client.send)
				log.Printf("Client %s unregistered. Total clients: %d", client.id, len(m.clients))
			}
			welcomeEvent := &Event{
				Type: "system",
				Payload: json.RawMessage(`{"message":"Welcome to the server","client_id":"` +
					client.id.String() + `"}`),
				Time: time.Now(),
			}
			client.send <- welcomeEvent

		case message := <-m.broadcast:
			for client := range m.clients {
				select {
				case client.send <- message:
				default:
					// Client's send buffer is full, remove it
					close(client.send)
					delete(m.clients, client)
					log.Printf("Client %s removed due to slow consumption", client.id)
				}
			}
		}
	}
}

// readPump handles incoming messages from the websocket connection
func (c *Client) readPump() {
	defer func() {
		c.mgmt.unregister <- c
		c.conn.Close()
	}()

	c.conn.SetReadLimit(maxMessageSize)
	c.conn.SetReadDeadline(time.Now().Add(pongWait))
	c.conn.SetPongHandler(func(string) error {
		c.conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	for {
		var event Event
		err := c.conn.ReadJSON(&event)
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("Read error from client %s: %v", c.id, err)
			}
			break
		}

		event.From = c.id
		event.Time = time.Now()

		log.Printf("Received from %s: type=%s, payload=%s", c.id, event.Type, string(event.Payload))

		// Route to appropriate handler
		if handler, ok := c.mgmt.handlers[event.Type]; ok {
			if err := handler(c, event); err != nil {
				log.Printf("Handler error for event type '%s': %v", event.Type, err)

				// Send error response back to client
				errorEvent := &Event{
					Type:    "error",
					Payload: json.RawMessage(`{"message":"` + err.Error() + `"}`),
					Time:    time.Now(),
				}
				c.send <- errorEvent
			}
		} else {
			log.Printf("No handler found for event type: %s", event.Type)

			// Broadcast unknown events by default
			c.mgmt.broadcast <- &event
		}
	}
}
