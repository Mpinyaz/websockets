package main

import (
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"

	. "websockets/internal/core"
	"websockets/internal/store/cache"
	"websockets/utils"
)

func main() {
	cfg, err := utils.LoadEnv()
	if err != nil {
		log.Fatal("Failed to load config:", err)
	}

	redisStore, err := cache.NewRedisClient(cfg)
	if err != nil {
		log.Printf(" Redis unavailable: %v (running stateless)", err)
		redisStore = nil
	} else {
		redisStore.Run()
		defer redisStore.Close()
	}

	var clientStore *cache.ClientStore
	if redisStore != nil {
		clientStore = cache.NewClientStore(redisStore)
	}

	manager := NewManager(cfg, clientStore)
	go manager.Run()

	// ------------------------------------------------------------
	// RabbitMQ Streams (market updates + subs)
	// ------------------------------------------------------------
	if err := InitMktStreams(manager); err != nil {
		log.Fatalf("Failed to initialize RabbitMQ streams: %v", err)
	}
	log.Println("RabbitMQ market streams connected")

	// ------------------------------------------------------------
	// HTTP
	// ------------------------------------------------------------
	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		ServeWS(manager, w, r)
	})

	http.HandleFunc("/health", HealthCheckHandler(manager))

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		redisEnabled := redisStore != nil

		w.Write([]byte(`{
			"message": "Market Data WebSocket Server",
			"endpoints": {
				"/ws": "WebSocket connection",
				"/health": "Health check"
			},
			"features": {
				"rabbitmq_streams": true,
				"redis_persistence": ` + boolToString(redisEnabled) + `
			}
		}`))
	})

	// ------------------------------------------------------------
	// Server
	// ------------------------------------------------------------
	port := cfg.AppPort
	if port == "" {
		port = "8080"
	}

	go func() {
		log.Printf("MDWS listening on :%s", port)
		if err := http.ListenAndServe(":"+port, nil); err != nil {
			log.Fatal(err)
		}
	}()

	log.Println("✅ Server ready")

	// ------------------------------------------------------------
	// Graceful shutdown
	// ------------------------------------------------------------
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, os.Interrupt, syscall.SIGTERM)
	<-sig

	log.Println("🛑 Shutting down gracefully")
	log.Println("👋 Server stopped")
}

func boolToString(v bool) string {
	if v {
		return "true"
	}
	return "false"
}
