package core

import (
	"encoding/json"
	"log"
	"net/http"
	"time"
)

var startTime = time.Now()

type HealthResponse struct {
	Status           string `json:"status,omitempty"`
	ConnectedClients int    `json:"connected_clients,omitempty"`
	Uptime           string `json:"uptime,omitempty"`
	Timestamp        string `json:"timestamp,omitempty"`
}

func HandleBroadcast(c *Client, event Event) error {
	log.Printf("Broadcasting message from %s: %s", c.ID, event.Payload)
	c.Mgmt.Broadcast <- &event
	return nil
}

func HandlePing(c *Client, event Event) error {
	log.Printf("Ping received from %s", c.ID)

	pongEvent := &Event{
		Type:    "pong",
		Payload: json.RawMessage(`{"timestamp":"` + time.Now().Format(time.RFC3339) + `"}`),
		Time:    time.Now(),
		From:    c.ID,
	}

	c.Send <- pongEvent
	return nil
}

// HealthCheckHandler exposes server health and websocket stats
func HealthCheckHandler(hub *Manager) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		uptime := time.Since(startTime)

		hub.Mu.RLock()
		clientCount := len(hub.Clients)
		hub.Mu.RUnlock()

		resp := HealthResponse{
			Status:           "ok",
			ConnectedClients: clientCount,
			Uptime:           uptime.String(),
			Timestamp:        time.Now().Format(time.RFC3339),
		}

		w.WriteHeader(http.StatusOK)
		_ = json.NewEncoder(w).Encode(resp)
	}
}

// ServeWS upgrades HTTP connections to WebSocket connections
func ServeWS(hub *Manager, w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("WebSocket upgrade failed: %v", err)
		return
	}

	client := &Client{
		Mgmt: hub,
		Conn: conn,
		Send: make(chan *Event, 256),
		ID:   hub.idNode.Generate(),
		Done: make(chan struct{}),
	}

	// Register client with manager
	hub.Register <- client

	// Start pumps
	go client.writePump()
	go client.readPump()
}
