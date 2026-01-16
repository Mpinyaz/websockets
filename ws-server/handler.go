package main

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
)

type (
	EventHandler   func(c *Client, event Event) error
	HealthResponse struct {
		Status           string `json:"status,omitempty"`
		ConnectedClients int    `json:"connected_clients,omitempty"`
		Uptime           string `json:"uptime,omitempty"`
		Timestamp        string `json:"timestamp,omitempty"`
	}
)

var startTime = time.Now()

func HandleBroadcast(c *Client, event Event) error {
	log.Printf("Broadcasting message from %s: %v", c.id, event.Payload)
	c.mgmt.broadcast <- &event
	return nil
}

func HandlePing(c *Client, event Event) error {
	log.Printf("Ping received from %s", c.id)

	pongEvent := &Event{
		Type:    "pong",
		Payload: json.RawMessage(`{"timestamp":"` + time.Now().Format(time.RFC3339) + `"}`),
		Time:    time.Now(),
	}

	c.send <- pongEvent
	return nil
}

func healthCheckHandler(hub *Manager) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		uptime := time.Since(startTime)

		response := HealthResponse{
			Status:           "ok",
			ConnectedClients: len(hub.clients),
			Uptime:           uptime.String(),
			Timestamp:        time.Now().Format(time.RFC3339),
		}

		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(response)
	}
}

// ServeWS handles websocket requests from clients
func ServeWS(hub *Manager, w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("Upgrade failed: %v", err)
		return
	}

	clientID := uuid.New()

	client := &Client{
		mgmt: hub,
		conn: conn,
		send: make(chan *Event, 256),
		id:   clientID,
	}

	client.mgmt.register <- client

	// Start read and write pumps in separate goroutines
	go client.writePump()
	go client.readPump()
}
