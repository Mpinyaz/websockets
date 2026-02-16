use anyhow::Result;
use mdworkers::config::cfg::Config;
use mdworkers::services::streams::get_stream_entities;
use mdworkers::types::client::Client;
use mdworkers::ws::actors::spawn_ws_actors;
use mdworkers::ws::handlers::run_sub_consumer;
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
