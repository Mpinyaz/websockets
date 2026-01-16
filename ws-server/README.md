# WebSocket Server

A real-time WebSocket server built with Go featuring event-based routing and JSON messaging.

## Features

- **Event-driven architecture** with pluggable handlers
- **Automatic client management** with connection health monitoring
- **Broadcast messaging** to all connected clients
- **JSON-based protocol** for structured communication
- **Ping/pong heartbeats** to detect dead connections
- **Graceful error handling** and client cleanup

## Quick Start

### Prerequisites

```bash
go mod init websocket-server
go get github.com/google/uuid
go get github.com/gorilla/websocket
```

### Run Server

```bash
go run .
```

Server starts on `http://localhost:8080`

## API Endpoints

| Endpoint | Type | Description |
|----------|------|-------------|
| `/ws` | WebSocket | Main WebSocket connection |
| `/health` | HTTP GET | Server health check |

## Event Types

### Broadcast Message
```json
{
  "type": "broadcast",
  "payload": {
    "text": "Hello everyone!"
  }
}
```

### Ping/Pong
```json
{
  "type": "pong",
  "payload": {}
}
```

Server responds with:
```json
{
  "type": "pong",
  "payload": {
    "timestamp": "2026-01-16T10:30:45Z"
  },
  "time": "2026-01-16T10:30:45Z"
}
```

## Testing

### Using websocat
```bash
# Install
brew install websocat

# Connect and send message
echo '{"type":"broadcast","payload":{"text":"Hello!"}}' | websocat ws://localhost:8080/ws
```

### Using Browser Console
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'broadcast',
    payload: { text: 'Hello!' }
  }));
};

ws.onmessage = (e) => {
  console.log('Received:', JSON.parse(e.data));
};
```

### Health Check
```bash
curl http://localhost:8080/health
```

## Adding Custom Event Handlers

```go
// In main.go
func main() {
    ws := NewManager()
    ws.setupEventHandlers()
    
    // Add custom handler
    ws.RegisterHandler("custom_event", func(c *Client, event Event) error {
        log.Printf("Custom event from %s", c.id)
        // Your logic here
        return nil
    })
    
    go ws.Run()
    // ... rest of setup
}
```

## Architecture

- **Manager**: Coordinates all clients and message routing
- **Client**: Represents a single WebSocket connection
- **Event Handlers**: Process specific event types
- **Read/Write Pumps**: Separate goroutines for sending and receiving messages

## Configuration

Constants in `manager.go`:
- `writeWait`: 10s - Max time to write message
- `pongWait`: 60s - Max time to wait for pong response
- `pingPeriod`: 54s - How often to send pings
- `maxMessageSize`: 8192 bytes - Max message size

## License

MIT