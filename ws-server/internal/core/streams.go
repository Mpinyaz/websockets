package core

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strings"
	"time"

	. "websockets/utils"

	influxdb2 "github.com/influxdata/influxdb-client-go/v2" // InfluxDB Client
	api "github.com/influxdata/influxdb-client-go/v2/api"   // InfluxDB API Client
	"github.com/rabbitmq/rabbitmq-stream-go-client/pkg/amqp"
	"github.com/rabbitmq/rabbitmq-stream-go-client/pkg/stream"
)

type MdwsStreams struct {
	env              *stream.Environment
	mktUpdates       *stream.Consumer
	mktSubscriptions *stream.Producer
	ctx              context.Context
	cancel           context.CancelFunc
	broadcastChan    chan *Event
	cfg              *Config
	// For InfluxDB direct writing
	influxClient   influxdb2.Client
	influxWriteAPI api.WriteAPI
}

// InitMktStreams initializes both consumer and producer for market data stream
func InitMktStreams(mgr *Manager) error {
	cfg := mgr.GetConfig()
	log.Printf("Initializing market data streams...")

	env, err := initStreamEnv(cfg)
	if err != nil {
		return fmt.Errorf("failed to create stream environment: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())
	streams := &MdwsStreams{
		env:    env,
		ctx:    ctx,
		cancel: cancel,
		cfg:    cfg,
	}

	// Initialize InfluxDB client
	streams.influxClient = influxdb2.NewClient(cfg.InfluxDBURL, cfg.InfluxDBToken)
	streams.influxWriteAPI = streams.influxClient.WriteAPI(cfg.InfluxDBOrg, cfg.InfluxDBBucket)

	// Check InfluxDB connection (optional, but good practice)
	_, err = streams.influxClient.Health(context.Background())
	if err != nil {
		streams.Close() // Clean up before returning error
		return fmt.Errorf("influxDB health check failed: %w", err)
	} else {
		log.Printf("InfluxDB client initialized for URL: %s, Org: %s, Bucket: %s", cfg.InfluxDBURL, cfg.InfluxDBOrg, cfg.InfluxDBBucket)
	}

	// Listen for write errors asynchronously
	go func() {
		for {
			select {
			case err := <-streams.influxWriteAPI.Errors():
				log.Printf("InfluxDB write error: %v", err.Error())
			case <-streams.ctx.Done():
				return
			}
		}
	}()

	// Proceed with creating consumer and producer directly
	var mktUpdates *stream.Consumer
	if mktUpdates, err = createUpdateConsumer(env, cfg, streams); err != nil {
		streams.Close() // Clean up before returning error
		return fmt.Errorf("update consumer creation failed: %w", err)
	}
	streams.mktUpdates = mktUpdates
	log.Printf("Market updates consumer created on stream '%s'", cfg.RmqMarketUpdate)

	var mktSubscriptions *stream.Producer
	if mktSubscriptions, err = createSubsProducer(env, cfg); err != nil {
		streams.Close() // Clean up before returning error
		return fmt.Errorf("subscription producer creation failed: %w", err)
	}
	streams.mktSubscriptions = mktSubscriptions
	log.Printf("Market subscriptions producer created on stream '%s'", cfg.RmqMarketSubs)

	// All successful!
	streams.broadcastChan = mgr.Broadcast
	mgr.SetStreams(streams)
	log.Printf("Market data streams initialized successfully")
	return nil // Success
}

// Close gracefully shuts down all stream connections
func (s *MdwsStreams) Close() error {
	log.Println("Closing market data streams and AMQP connections...")

	// Cancel context to stop any background operations
	s.cancel()

	var errs []error

	// Close consumer
	if s.mktUpdates != nil {
		if err := s.mktUpdates.Close(); err != nil {
			errs = append(errs, fmt.Errorf("consumer close error: %w", err))
		} else {
			log.Println("Market updates consumer closed")
		}
	}

	// Close producer
	if s.mktSubscriptions != nil {
		if err := s.mktSubscriptions.Close(); err != nil {
			errs = append(errs, fmt.Errorf("producer close error: %w", err))
		} else {
			log.Println("Market subscriptions producer closed")
		}
	}

	// Close environment
	if s.env != nil {
		if err := s.env.Close(); err != nil {
			errs = append(errs, fmt.Errorf("environment close error: %w", err))
		} else {
			log.Println("Stream environment closed")
		}
	}

	// Close InfluxDB client
	if s.influxClient != nil {
		s.influxWriteAPI.Flush() // Ensure all buffered points are written
		s.influxClient.Close()
		log.Println("InfluxDB client closed")
	}

	if len(errs) > 0 {
		return fmt.Errorf("errors during close: %v", errs)
	}

	log.Println("Market data streams closed successfully")
	return nil
}

// initStreamEnv creates the RabbitMQ Stream environment
func initStreamEnv(cfg *Config) (*stream.Environment, error) {
	log.Printf("Connecting to RabbitMQ Stream at %s:%d", cfg.RmqHost, cfg.RmqPort)

	env, err := stream.NewEnvironment(
		stream.NewEnvironmentOptions().
			SetHost(cfg.RmqHost).
			SetPort(cfg.RmqPort).
			SetUser(cfg.RmqUsername).
			SetPassword(cfg.RmqPassword),
	)
	if err != nil {
		return nil, err
	}

	log.Printf("Successfully connected to RabbitMQ Stream")
	return env, nil
}

func createUpdateConsumer(
	env *stream.Environment,
	cfg *Config,
	streams *MdwsStreams,
) (*stream.Consumer, error) {
	return env.NewConsumer(
		cfg.RmqMarketUpdate,
		func(ctx stream.ConsumerContext, msg *amqp.Message) {
			handleMktUpdate(ctx, msg, streams)
		},
		stream.NewConsumerOptions().
			SetConsumerName("mdws-market-consumer").
			SetOffset(stream.OffsetSpecification{}.First()),
	)
}

func handleMktUpdate(
	ctx stream.ConsumerContext,
	msg *amqp.Message,
	streams *MdwsStreams,
) {
	select {
	case <-streams.ctx.Done():
		return
	default:
	}

	data := msg.GetData()
	var payload TiingoResponse
	if err := json.Unmarshal(data, &payload); err != nil {
		log.Printf("Market update decode error: %v", err)
		return
	}

	var eventPayload interface{} // Will hold parsed data for Event
	var assetClass string        // Holds the asset class for market data updates

	switch payload.MessageType {
	case "I": // Info
		if payload.Response != nil {
			log.Printf("Tiingo Info - Code: %d, Message: %s",
				payload.Response.Code, payload.Response.Message)
		}
		if payload.Data != nil {
			log.Printf("Subscription data: %v", payload.Data)
		}

	case "H": // Heartbeat
		log.Println("Tiingo heartbeat received")

	case "A": // Actual market update
		// Tiingo still uses arrays for FX and Crypto
		if payload.Service == "fx" || payload.Service == "crypto_data" {
			var arr []interface{}
			if err := json.Unmarshal(payload.Data, &arr); err != nil {
				log.Printf("expected array payload for service=%s, got %s", payload.Service, string(payload.Data))
				return
			}

			switch payload.Service {
			case "fx":
				var fx TiingoForexData
				if err := parseForexArray(arr, &fx); err != nil {
					log.Printf("Forex parse error: %v", err)
					return
				}
				eventPayload = fx
				assetClass = "forex"

			case "crypto_data":
				var c TiingoCryptoData
				if err := parseCryptoArray(arr, &c); err != nil {
					log.Printf("Crypto parse error: %v", err)
					return
				}
				eventPayload = c
				assetClass = "crypto_data"
			}
		} else if payload.Service == "alpaca-iex" {
			var eq EquityUpdate
			isUpdate, err := parseAlpacaObject(payload.Data, &eq)
			if err != nil {
				log.Printf("Alpaca parse error: %v", err)
				return
			}
			if isUpdate {
				eventPayload = eq
				assetClass = "equity"
			}
		} else {
			log.Printf("Unknown service in update: %s", payload.Service)
		}

	case "E": // Error
		if payload.Response != nil {
			log.Printf("Tiingo ERROR - Code: %d, Message: %s",
				payload.Response.Code, payload.Response.Message)
		}
		if payload.Data != nil {
			log.Printf("Error details: %v", payload.Data)
		}

	default:
		log.Printf("Unknown Tiingo message type: %s, full payload: %+v",
			payload.MessageType, payload)
	}

	// Create Event only if we have parsed data (for "A" type)
	if eventPayload != nil {
		var measurement string
		tags := make(map[string]string)
		fields := make(map[string]interface{})
		var timestamp time.Time

		switch v := eventPayload.(type) {
		case TiingoForexData:
			measurement = "forex"
			tags["ticker"] = v.Ticker
			tags["update_type"] = v.Type
			fields["bid_size"] = v.BidSize
			fields["bid_price"] = v.BidPrice
			fields["mid_price"] = v.MidPrice
			fields["ask_price"] = v.AskPrice
			fields["ask_size"] = v.AskSize
			parsedTime, err := time.Parse("2006-01-02T15:04:05.999999Z07:00", v.Timestamp)
			if err != nil {
				log.Printf("Forex timestamp parse error for ticker %s: %v", v.Ticker, err)
				timestamp = time.Now()
			} else {
				timestamp = parsedTime
			}
		case TiingoCryptoData:
			measurement = "crypto_data"
			tags["ticker"] = v.Ticker
			tags["exchange"] = v.Exchange
			tags["update_type"] = v.UpdateType
			fields["last_size"] = v.LastSize
			fields["last_price"] = v.LastPrice
			timestamp = v.Date
		case EquityUpdate:
			measurement = "equity"
			tags["ticker"] = v.Ticker
			tags["update_type"] = v.UpdateType

			if v.BidSize != nil {
				fields["bid_size"] = *v.BidSize
			}
			if v.BidPrice != nil {
				fields["bid_price"] = *v.BidPrice
			}
			if v.MidPrice != nil {
				fields["mid_price"] = *v.MidPrice
			}
			if v.AskPrice != nil {
				fields["ask_price"] = *v.AskPrice
			}
			if v.AskSize != nil {
				fields["ask_size"] = *v.AskSize
			}
			if v.LastPrice != nil {
				fields["last_price"] = *v.LastPrice
			}
			if v.LastSize != nil {
				fields["last_size"] = *v.LastSize
			}
			if v.Open != nil {
				fields["open"] = *v.Open
			}
			if v.High != nil {
				fields["high"] = *v.High
			}
			if v.Low != nil {
				fields["low"] = *v.Low
			}
			if v.Close != nil {
				fields["close"] = *v.Close
			}
			if v.Volume != nil {
				fields["volume"] = *v.Volume
			}
			if v.TradeID != nil {
				fields["tradeId"] = *v.TradeID
			}
			if v.TradeCount != nil {
				fields["tradeCount"] = *v.TradeCount
			}
			if v.VWAP != nil {
				fields["vwap"] = *v.VWAP
			}

			if v.Exchange != "" {
				tags["exchange"] = v.Exchange
			}
			if v.Tape != "" {
				tags["tape"] = v.Tape
			}
			if len(v.Conditions) > 0 {
				tags["conditions"] = fmt.Sprintf("%v", v.Conditions)
			}

			fields["nanoseconds"] = v.Nanos
			timestamp = v.Date
		default:
			log.Printf("Unknown eventPayload type: %+v", eventPayload)
			return
		}

		point := influxdb2.NewPoint(measurement, tags, fields, timestamp)
		streams.influxWriteAPI.WritePoint(point)

		eventBytes, err := json.Marshal(eventPayload)
		if err != nil {
			log.Printf("Failed to marshal event payload: %v", err)
			return
		}

		event := Event{
			Type:       "market_update",
			Payload:    eventBytes,
			Time:       time.Now(),
			AssetClass: assetClass,
		}

		select {
		case streams.broadcastChan <- &event:
			// Successfully sent
		case <-time.After(100 * time.Millisecond):
			// log.Println("⚠️ Broadcast channel timeout, dropping market update")
		case <-streams.ctx.Done():
			return
		}
	}
}

// ------------------------------------------------------------
// Subscription producer
// ------------------------------------------------------------

func createSubsProducer(
	env *stream.Environment,
	cfg *Config,
) (*stream.Producer, error) {
	return env.NewProducer(
		cfg.RmqMarketSubs,
		stream.NewProducerOptions().
			SetProducerName("mdws-subscription-producer"),
	)
}

// PublishSubscription publishes a subscription event to the stream
func (s *MdwsStreams) PublishSubscription(event *Event) error {
	select {
	case <-s.ctx.Done():
		return fmt.Errorf("streams are shutting down")
	default:
	}

	data, err := json.Marshal(event)
	if err != nil {
		return fmt.Errorf("failed to marshal event: %w", err)
	}

	msg := amqp.NewMessage(data)
	msg.Properties = &amqp.MessageProperties{
		ContentType: "application/json",
	}

	err = s.mktSubscriptions.Send(msg)
	if err != nil {
		return fmt.Errorf("failed to send message: %w", err)
	}

	return nil
}

// ----------------------------
// Forex parser
// ----------------------------
func parseForexArray(raw []interface{}, fx *TiingoForexData) error {
	if len(raw) < 8 {
		return fmt.Errorf("expected at least 8 fields for forex, got %d", len(raw))
	}

	// Type assertion helpers
	typeAssert := func(i int) (string, error) {
		if s, ok := raw[i].(string); ok {
			return s, nil
		}
		return "", fmt.Errorf("field %d is not a string", i)
	}

	floatAssert := func(i int) (float64, error) {
		if f, ok := raw[i].(float64); ok {
			return f, nil
		}
		return 0, fmt.Errorf("field %d is not a float64", i)
	}

	var err error
	if fx.Type, err = typeAssert(0); err != nil {
		return err
	}
	if fx.Ticker, err = typeAssert(1); err != nil {
		return err
	}
	if fx.Timestamp, err = typeAssert(2); err != nil {
		return err
	}
	if fx.BidSize, err = floatAssert(3); err != nil {
		return err
	}
	if fx.BidPrice, err = floatAssert(4); err != nil {
		return err
	}
	if fx.MidPrice, err = floatAssert(5); err != nil {
		return err
	}
	if fx.AskPrice, err = floatAssert(6); err != nil {
		return err
	}
	if fx.AskSize, err = floatAssert(7); err != nil {
		return err
	}

	return nil
}

// ----------------------------
// Crypto parser
// ----------------------------
func parseCryptoArray(data []interface{}, out *TiingoCryptoData) error {
	if len(data) < 6 {
		return fmt.Errorf("crypto array too short, got %d elements", len(data))
	}

	// Index 0: UpdateType ("T" for Trade, "Q" for Quote)
	updateType, ok := data[0].(string)
	if !ok {
		return fmt.Errorf("crypto update type is not a string")
	}
	out.UpdateType = updateType

	// Index 1: Ticker
	if s, ok := data[1].(string); ok {
		out.Ticker = s
	}

	// Index 2: Date
	if s, ok := data[2].(string); ok {
		t, err := time.Parse(time.RFC3339Nano, s)
		if err != nil {
			return fmt.Errorf("invalid crypto date: %w", err)
		}
		out.Date = t
	}

	// Index 3: Exchange
	if s, ok := data[3].(string); ok {
		out.Exchange = s
	}

	if updateType == "T" {
		// Index 4: LastSize
		if f, ok := data[4].(float64); ok {
			out.LastSize = f
		}
		// Index 5: LastPrice
		if f, ok := data[5].(float64); ok {
			out.LastPrice = f
		}
	} else if updateType == "Q" {
		// For Quotes, Tiingo sends: ["Q", ticker, date, exchange, bidSize, bidPrice, midPrice, askSize, askPrice]
		// We'll map midPrice to LastPrice for simplified display in the current TickerTable
		if len(data) >= 9 {
			if f, ok := data[6].(float64); ok {
				out.LastPrice = f
			}
			if f, ok := data[4].(float64); ok {
				out.LastSize = f // Using bidSize as a proxy for size in the simple struct
			}
		}
	}

	return nil
}

// ----------------------------
// Alpaca parser
// ----------------------------
func parseAlpacaObject(data []byte, eq *EquityUpdate) (bool, error) {
	var rawMap map[string]json.RawMessage
	if err := json.Unmarshal(data, &rawMap); err != nil {
		return false, fmt.Errorf("failed to unmarshal Alpaca payload into raw map: %w", err)
	}

	alpacaTypeRaw, ok := rawMap["T"]
	if !ok {
		return false, fmt.Errorf("Alpaca payload missing 'T' (type) field: %s", string(data))
	}

	var alpacaType string
	if err := json.Unmarshal(alpacaTypeRaw, &alpacaType); err != nil {
		return false, fmt.Errorf("failed to unmarshal Alpaca message type 'T': %w", err)
	}

	eq.UpdateType = strings.ToUpper(alpacaType)

	var timestampStr string
	timestampRaw, ok := rawMap["t"]
	if ok {
		if err := json.Unmarshal(timestampRaw, &timestampStr); err != nil {
			log.Printf("failed to unmarshal Alpaca timestamp 't': %v", err)
			// Proceed without timestamp if it cannot be parsed
		}
	}

	switch alpacaType {
	case "t": // Trade
		var trade AlpacaTrade
		if err := json.Unmarshal(data, &trade); err != nil {
			return false, fmt.Errorf("failed to parse Alpaca trade: %w", err)
		}
		eq.Ticker = trade.Symbol
		eq.LastPrice = &trade.Price
		eq.LastSize = &trade.Size
		eq.TradeID = &trade.TradeID
		eq.Exchange = trade.Exchange
		eq.Tape = trade.Tape
		eq.Conditions = trade.Conditions
		// Use the timestampStr explicitly extracted
		if timestampStr == "" { // Fallback if explicit extraction failed
			timestampStr = trade.Timestamp
		}

	case "q": // Quote
		var quote AlpacaQuote
		if err := json.Unmarshal(data, &quote); err != nil {
			return false, fmt.Errorf("failed to parse Alpaca quote: %w", err)
		}
		eq.Ticker = quote.Symbol
		eq.BidPrice = &quote.BidPrice
		eq.BidSize = &quote.BidSize
		eq.AskPrice = &quote.AskPrice
		eq.AskSize = &quote.AskSize
		eq.BidExchange = quote.BidExchange
		eq.AskExchange = quote.AskExchange
		eq.Conditions = quote.Conditions
		eq.Tape = quote.Tape
		// Use the timestampStr explicitly extracted
		if timestampStr == "" { // Fallback if explicit extraction failed
			timestampStr = quote.Timestamp
		}
		if eq.BidPrice != nil && eq.AskPrice != nil {
			mid := (*eq.BidPrice + *eq.AskPrice) / 2
			eq.MidPrice = &mid
		}
	case "b": // Bar
		var bar AlpacaBar
		if err := json.Unmarshal(data, &bar); err != nil {
			return false, fmt.Errorf("failed to parse Alpaca bar: %w", err)
		}
		eq.Ticker = bar.Symbol
		eq.Open = &bar.Open
		eq.High = &bar.High
		eq.Low = &bar.Low
		eq.Close = &bar.Close
		eq.Volume = &bar.Volume
		eq.TradeCount = &bar.TradeCount
		eq.VWAP = &bar.VWAP
		// Use the timestampStr explicitly extracted
		if timestampStr == "" { // Fallback if explicit extraction failed
			timestampStr = bar.Timestamp
		}
	case "d", "u", "c", "x", "l", "s", "i": // Other known message types
		log.Printf("Received known but unhandled Alpaca message type: %s", alpacaType)
		return false, nil // Not an error, not a data update
	default:
		return false, fmt.Errorf("unknown Alpaca message type: %s", alpacaType)
	}

	// This part now only runs for t, q, b
	parsedTime, err := time.Parse(time.RFC3339Nano, timestampStr)
	if err != nil {
		log.Printf("could not parse Alpaca timestamp '%s': %v", timestampStr, err)
	} else {
		eq.Date = parsedTime
		eq.Nanos = parsedTime.UnixNano()
	}

	return true, nil
}
