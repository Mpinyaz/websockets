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
	// Load configuration
	cfg, err := utils.LoadEnv()
	if err != nil {
		log.Fatal("Failed to load config:", err)
	}

	// Initialize Redis client
	redisStore, err := cache.NewRedisClient(cfg)
	if err != nil {
		log.Printf("Warning: Redis connection failed: %v. Running without persistence.", err)
		redisStore = nil
	} else {
		redisStore.Run()
		defer redisStore.Close()
	}

	// Initialize client store
	var clientStore *cache.ClientStore
	if redisStore != nil {
		clientStore = cache.NewClientStore(redisStore)
	}

	// Create manager with Redis support
	manager := NewManager(cfg, clientStore)

	// Initialize Tiingo WebSocket client if API key is available
	tiingoAPIKey := cfg.TiingoAPIKey
	if tiingoAPIKey != "" {
		if err := InitTiingoClient(tiingoAPIKey, manager); err != nil {
			log.Printf("Failed to initialize Tiingo: %v", err)
		} else {
			log.Println("✅ Tiingo WebSocket client initialized")
		}
	} else {
		log.Println("Warning: TIINGO_API_KEY not set. Tiingo features disabled.")
	}

	go manager.Run()
	// HTTP handlers
	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		ServeWS(manager, w, r)
	})

	http.HandleFunc("/health", HealthCheckHandler(manager))

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		tiingoEnabled := "false"
		if tiingoAPIKey != "" {
			tiingoEnabled = "true"
		}

		redisEnabled := "false"
		if redisStore != nil {
			redisEnabled = "true"
		}

		response := `{
			"message": "WebSocket Real-time Data Server",
			"endpoints": {
				"/ws": "WebSocket connection",
				"/health": "Health check"
			},
			"supported_events": [
				"broadcast",
				"ping",
				"subscribe",
				"unsubscribe",
				"patch_subscribe",
				"forex_subscribe",
				"forex_unsubscribe"
			],
			"features": {
				"tiingo_websocket": ` + tiingoEnabled + `,
				"redis_persistence": ` + redisEnabled + `
			}
		}`

		w.Write([]byte(response))
	})

	// Get port from config
	port := cfg.AppPort
	if port == "" {
		port = "8080"
	}

	// Start HTTP server
	go func() {
		log.Printf("🚀 WebSocket server starting on :%s", port)
		log.Println("📊 Real-time forex data powered by Tiingo")
		if err := http.ListenAndServe(":"+port, nil); err != nil {
			log.Fatal("ListenAndServe: ", err)
		}
	}()

	log.Println("✅ Server is ready. Press Ctrl+C to stop")

	// Graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)
	<-sigChan

	log.Println("\n🛑 Shutting down gracefully...")

	// Close Tiingo connection
	if GetTiingoClient() != nil {
		log.Println("Closing Tiingo connection...")
		GetTiingoClient().Close()
	}

	// Close Redis connection
	if redisStore != nil {
		log.Println("Closing Redis connection...")
		redisStore.Close()
	}

	log.Println("👋 Server stopped")
}
