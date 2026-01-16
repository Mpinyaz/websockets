package main

import (
	"log"
	"net/http"
)

func main() {
	ws := NewManager()
	go ws.Run()

	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		ServeWS(ws, w, r)
	})

	http.HandleFunc("/health", healthCheckHandler(ws))

	log.Println("WebSocket server starting on :8080")
	if err := http.ListenAndServe(":8080", nil); err != nil {
		log.Fatal("ListenAndServe: ", err)
	}
}
