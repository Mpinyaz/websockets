use anyhow::Result;
use mdworkers::services::streams::get_stream_entities;
use mdworkers::types::assetclass::AssetClass;
use mdworkers::types::message::deserialize_msg;
use mdworkers::ws::handlers::outbound_msg_handler;
use mdworkers::{config::cfg::Config, types::client::Client};
use tracing::{error, info, warn};

#[tokio::main]
pub async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = Config::load_env()?;

    info!("Initializing RabbitMQ streams...");
    let _stream_entities = get_stream_entities().await?;

    info!("Connecting to WebSocket feeds...");
    let client = Client::new(&cfg).await?;

    let forex_client = Client {
        api_key: cfg.data_api_key.clone(),
        fx_ws: client.fx_ws,
        crypto_ws: None,
        equity_ws: None,
    };

    let crypto_client = Client {
        api_key: cfg.data_api_key.clone(),
        fx_ws: None,
        crypto_ws: client.crypto_ws,
        equity_ws: None,
    };

    let equity_client = Client {
        api_key: cfg.data_api_key.clone(),
        fx_ws: None,
        crypto_ws: None,
        equity_ws: client.equity_ws,
    };

    tokio::spawn(async move {
        if let Err(e) = outbound_msg_handler(forex_client, AssetClass::Forex, deserialize_msg).await
        {
            error!("Forex handler crashed: {}", e);
        }
    });

    tokio::spawn(async move {
        if let Err(e) =
            outbound_msg_handler(crypto_client, AssetClass::Crypto, deserialize_msg).await
        {
            error!("Crypto handler crashed: {}", e);
        }
    });

    tokio::spawn(async move {
        if let Err(e) =
            outbound_msg_handler(equity_client, AssetClass::Equity, deserialize_msg).await
        {
            error!("Equity handler crashed: {}", e);
        }
    });

    info!("Market data workers started");

    tokio::signal::ctrl_c().await?;
    warn!("Shutdown signal received");

    Ok(())
}
