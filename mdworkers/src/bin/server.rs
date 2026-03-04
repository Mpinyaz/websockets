use anyhow::Result;
use mdworkers::config::Config;
use mdworkers::streams::get_stream_entities;
use mdworkers::streams::handlers::run_sub_consumer;
use mdworkers::types::client::Client;
use mdworkers::ws::spawn_ws_actors;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Market Data Workers...");

    // Load configuration
    let cfg = Config::load_env()?;

    // Initialize RabbitMQ streams
    info!("Initializing RabbitMQ streams...");
    let _stream_entities = get_stream_entities().await?;
    info!("✅ RabbitMQ streams initialized");

    // Create WebSocket client
    info!("Connecting to WebSocket feeds...");
    let client = Client::new(&cfg).await?;
    info!("✅ WebSocket connections established");

    // Spawn WebSocket actors and get command channels
    info!("Spawning WebSocket actors...");
    let ws_channels = spawn_ws_actors(client, &cfg).await;
    info!("✅ WebSocket actors spawned");

    // Start subscription consumer
    info!("Starting subscription consumer...");
    tokio::spawn(async move {
        if let Err(e) = run_sub_consumer(ws_channels).await {
            error!("❌ Subscription consumer error: {}", e);
        }
    });

    info!("✅ All systems operational");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    info!("🛑 Shutdown signal received, stopping...");

    Ok(())
}
