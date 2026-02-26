use tracing_error::ErrorLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::types::ApiError;
use redis::aio::ConnectionManager;

pub struct AppConfig {
    pub log: String,
    pub db_url: String,
    pub db_token: String,
    pub redis_url: String,
    pub db_bucket: String,
    pub app_url: String,
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let log = std::env::var("RUST_LOG").unwrap_or_else(|_| {
            "debug,mdanalytics=debug,tower_http=debug,axum::rejection=info".to_owned()
        });
        let db_url = std::env::var("INFLUXDB_URL").expect("database url is required");
        let db_token = std::env::var("INFLUXDB_TOKEN").expect("Token is rquired");
        let redis_url = std::env::var("REDIS_ADDRS").expect("Redis url is rquired");
        let db_bucket = std::env::var("INFLUXDB_BUCKET").expect("InfluxDB bucket is required");
        let app_url = std::env::var("MDANALYTICS_URL").expect("App url is required");
        Ok(AppConfig {
            log,
            db_url,
            db_token,
            redis_url,
            db_bucket,
            app_url,
        })
    }
}

pub fn init_tracing(default_log_level: String) {
    // initialize tracing
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(ErrorLayer::default())
        .init();
}

pub async fn init_redis_conn(url: &str) -> Result<ConnectionManager, ApiError> {
    let client = redis::Client::open(url).map_err(|e| {
        println!("Failed to create Redis client: {e}");
        ApiError::Redis(e)
    })?;

    ConnectionManager::new(client)
        .await
        .map_err(ApiError::Redis)
}
