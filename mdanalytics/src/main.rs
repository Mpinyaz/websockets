pub mod api;
pub mod types;
pub mod utils;
use crate::utils::init_redis_conn;
use crate::utils::init_tracing;
use crate::utils::AppConfig;
use axum::{routing::post, Router};
use influxdb::Client;
use crate::types::WebAppState;
use redis::aio::ConnectionManager;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = AppConfig::load()?;

    init_tracing(cfg.log);

    let client = Client::new(&cfg.db_url, &cfg.db_bucket).with_token(cfg.db_token);
    info!("Successfully connected to influx db: {}", &cfg.db_url);

    let redis: ConnectionManager = init_redis_conn(&cfg.redis_url)
        .await
        .expect("Failed to connect to Redis");
    info!("Succesfully connected to redis client: {}", &cfg.redis_url);

    let state = WebAppState { db: client, redis };

    let app = Router::new()
        .route("/submit/job", post(api::submit_job))
        .route("/analyze/{job_id}/stream", post(api::sse_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(cfg.app_url).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
