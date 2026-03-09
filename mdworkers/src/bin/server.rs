use anyhow::Result;
use mdworkers::config::Config;
use mdworkers::streams::get_stream_entities;
use mdworkers::streams::handlers::run_sub_consumer;
use mdworkers::types::client::Client;
use mdworkers::ws::spawn_ws_actors;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Market Data Workers...");

    let cfg = Config::load_env()?;

    info!("Initializing RabbitMQ streams...");
    let _stream_entities = get_stream_entities().await?;
    info!("✅ RabbitMQ streams initialized");

    info!("Connecting to WebSocket feeds...");
    let client = Client::new(&cfg).await?;
    info!("✅ WebSocket connections established");

    info!("Spawning WebSocket actors...");
    let ws_channels = spawn_ws_actors(client, &cfg).await;
    info!("✅ WebSocket actors spawned");

    info!("Starting subscription consumer...");
    tokio::spawn(async move {
        if let Err(e) = run_sub_consumer(ws_channels).await {
            error!("❌ Subscription consumer error: {}", e);
        }
    });

    info!("✅ All systems operational");

    tokio::signal::ctrl_c().await?;

    info!("🛑 Shutdown signal received, stopping...");

    Ok(())
}
