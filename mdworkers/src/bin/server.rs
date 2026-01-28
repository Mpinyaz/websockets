use anyhow::Result;
use mdworkers::types::assetclass::AssetClass;
use mdworkers::{config::cfg::Config, types::client::Client};
use tracing::{info, warn};

#[tokio::main]
pub async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = Config::load_env()?;

    let client = Client::new(&cfg).await;

    Ok(())
}
