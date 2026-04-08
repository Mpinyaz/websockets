# Market Data Workers (MDWS)

A high-performance real-time market data streaming system built with Rust,
featuring WebSocket subscriptions, Redis Clusters and RabbitMQ Stream
publishing.

## Overall Architecture

The Market Data WebSocket (MDWS) platform is a distributed system designed for
ingesting, processing, and distributing real-time market data (Crypto, Forex,
Equities) to clients. It leverages a microservices approach with specialized
components for each stage of the data pipeline.

Key architectural principles include dynamic upstream subscriptions,
high-throughput message routing, and scalable WebSocket server pods.

### High-Level Data Flow

```
Client -> WebSocket Pods (per Asset Class) -> Redis (Subscription Mgmt) -> Third-Party WS Feeds
                                           -> RabbitMQ Streams (Publishing/Consumption)
```

For a detailed data flow and component breakdown, please refer to the specific
documentation for each service.

## Technology Stack

The MDWS platform is built using a diverse set of technologies, each chosen for
its specific role in ensuring performance, scalability, and reliability:

| Component                   | Tool(s)                         | Purpose                                                 |
| :-------------------------- | :------------------------------ | :------------------------------------------------------ |
| **Market Data Ingestion**   | Tiingo WebSocket API            | Source for live Crypto, Forex, and Equity data          |
| **WebSocket Servers**       | Go (ws-server), Node.js, Python | Handles client connections and fan-out of data          |
| **Subscription Management** | Redis Cluster                   | Tracks upstream subscriptions and last snapshot values  |
| **Message Broker**          | RabbitMQ Stream                 | High-throughput message routing and buffering           |
| **Data Analytics**          | Rust (mdanalytics)              | Performs real-time analytics on market data streams     |
| **Workers**                 | Rust (mdworkers)                | Processes and publishes market data to RabbitMQ streams |
| **Frontend**                | React, TypeScript, Bun          | User interface for displaying real-time market data     |
| **Orchestration**           | Kubernetes                      | Manages and scales containerized services               |
| **Monitoring**              | Prometheus + Grafana            | Metrics collection and visualization                    |
| **Logging**                 | Zap (Go), Pino (Node.js)        | Structured logging for debugging and tracing            |

## Features

- **Multi-Asset Class Support**: Forex, Crypto, and Equities via separate
  WebSocket connections
- **Event-Driven Architecture**: Asynchronous message processing with Tokio
- **RabbitMQ Stream Integration**: High-throughput message publishing with
  persistent streams
- **Type-Safe Messaging**: Strongly-typed message parsing with Serde
- **Configurable Subscriptions**: Dynamic ticker subscriptions per asset class
- **Graceful Error Handling**: Connection resilience with automatic error
  recovery
- **Production-Ready Logging**: Structured logging with tracing
