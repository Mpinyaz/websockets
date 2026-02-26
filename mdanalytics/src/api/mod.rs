use crate::models::{AnalyzeRequest, AnalyzeResponse, ApiError, AssetClass, JobEvent};
use crate::AppState;
use axum::Json;
use axum::{
    extract::{Path, State},
    response::{sse::Event, Sse},
};
use futures_util::future;
use futures_util::stream::{Stream, StreamExt};
use influxdb::ReadQuery;
use mdanalytics::dataframe_to_json_value;
use mdanalytics::json_to_dataframe;
use polars::prelude::*;
use redis::AsyncCommands;
use serde_json::Value; // Added this line back
use snowflake_me::Snowflake;
use std::{convert::Infallible, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

pub async fn submit_job() -> Result<Json<serde_json::Value>, ApiError> {
    let sf = Snowflake::new()?;

    let job_id = sf.next_id()?;
    Ok(Json(serde_json::json!({
        "job_id": job_id,
        "stream_url": format!("/analyze/{job_id}/stream")
    })))
}

pub async fn sse_handler(
    Path(job_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<AnalyzeRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("SSE handler started for job: {}", job_id); // Added this line
    let (tx, rx) = broadcast::channel::<JobEvent>(64);
    tokio::spawn(async move {
        // Clone AppState for the spawned task
        if let Err(e) = handle_analyze(tx.clone(), payload, state.clone(), job_id.clone()).await {
            // job_id.clone() added
            eprintln!("Error in handle_analyze for job {}: {:?}", job_id, e);
            // Optionally, send a failed event to the client
            tx.send(JobEvent::Failed {
                error: format!("Analysis failed: {}", e),
            })
            .unwrap();
        }
    });

    let stream = BroadcastStream::new(rx).filter_map(|msg| {
        future::ready(match msg {
            // Wrapped in future::ready
            Ok(event) => {
                let name = match &event {
                    JobEvent::Processing => "processing",
                    JobEvent::Completed { .. } => "completed",
                    JobEvent::Failed { .. } => "failed",
                };
                let json_data = match &event {
                    JobEvent::Processing => serde_json::json!({"status": "processing"}),
                    JobEvent::Completed { result } => serde_json::json!({"status": "completed", "result": result}),
                    JobEvent::Failed { error } => serde_json::json!({"status": "failed", "error": error}),
                };
                let data = serde_json::to_string(&json_data).unwrap();
                Some(Ok(Event::default().event(name).data(data)))
            }
            Err(_) => None,
        })
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

// Constants for Redis keys
const JOB_STATUS_PREFIX: &str = "job_status:";
const JOB_RESULT_PREFIX: &str = "job_result:";

// Helper function to update job status in Redis
async fn update_job_status(
    redis_conn: &mut redis::aio::ConnectionManager,
    job_id: &str,
    status: &str,
) -> Result<(), ApiError> {
    let key = format!("{}{}", JOB_STATUS_PREFIX, job_id);
    redis_conn
        .set::<_, _, ()>(&key, status)
        .await
        .map_err(ApiError::Redis)?;
    Ok(())
}

// Helper function to store job result in Redis as JSON
async fn store_job_result(
    redis_conn: &mut redis::aio::ConnectionManager,
    job_id: &str,
    result: &serde_json::Value,
) -> Result<(), ApiError> {
    let key = format!("{}{}", JOB_RESULT_PREFIX, job_id);
    let json_string = serde_json::to_string(result)?;

    redis_conn
        .set::<_, _, ()>(&key, json_string)
        .await
        .map_err(ApiError::Redis)?;
    Ok(())
}

// Extracted data fetching and processing logic for reuse

async fn handle_analyze(
    tx: broadcast::Sender<JobEvent>,
    payload: AnalyzeRequest,
    state: AppState,
    job_id: String,
) -> Result<(), ApiError> {
    info!("Handling analysis for job: {}", job_id);

    // Clone redis connection for use in this function
    let mut redis_conn = state.redis.clone();

    // Set initial status to Processing in Redis
    update_job_status(&mut redis_conn, &job_id, "Processing").await?;
    tx.send(JobEvent::Processing).unwrap();

    let result = (async {
        let mut analysis_response = fetch_and_process_data(&state.db, payload.clone()).await?;
        analysis_response.job_id = job_id.clone();
        let final_analysis_result = serde_json::to_value(&analysis_response)?;
        Ok(final_analysis_result)
    })
    .await;

    match result {
        Ok(final_analysis_result) => {
            // Set status to Completed and store result in Redis
            update_job_status(&mut redis_conn, &job_id, "Completed").await?;
            store_job_result(&mut redis_conn, &job_id, &final_analysis_result).await?;
            tx.send(JobEvent::Completed {
                result: final_analysis_result,
            })
            .unwrap();
            Ok(())
        }
        Err(e) => {
            // Set status to Failed in Redis
            update_job_status(&mut redis_conn, &job_id, &format!("Failed: {}", e)).await?;
            tx.send(JobEvent::Failed {
                error: format!("Analysis failed: {}", e),
            })
            .unwrap();
            Err(e) // Propagate the error
        }
    }
}

async fn fetch_and_process_data(
    db_client: &influxdb::Client,
    payload: AnalyzeRequest,
) -> Result<AnalyzeResponse, ApiError> {
    if payload.tickers.is_empty() {
        return Ok(AnalyzeResponse {
            job_id: "".to_string(), // This will be filled later, or handled differently if no data.
            asset_class: payload.asset_class.to_string(),
            data: Vec::new(),
        });
    }

    let ticker_filter = payload
        .tickers
        .iter()
        .map(|t| format!("ticker = '{}'", t))
        .collect::<Vec<String>>()
        .join(" OR ");

    let query = format!(
        "SELECT * FROM {} WHERE {}",
        payload.asset_class, ticker_filter
    );

    // info!("Executing InfluxDB query: {}", query_string); // Debug log
    info!("Executing InfluxDB query: {}", query); // Debug log
    let raw_data = db_client.query(ReadQuery::new(query)).await?;
    info!("Raw data from InfluxDB: {:?}", raw_data); // Added for debugging

    let df = match payload.asset_class {
        // Match directly on AssetClass enum
        AssetClass::Crypto => json_to_dataframe(&raw_data, &["last_price", "last_size"])?,
        AssetClass::Forex => {
            json_to_dataframe(&raw_data, &["bid_price", "ask_price", "mid_price"])?
        }
        AssetClass::Equity => todo!(),
    };

    if df.height() == 0 {
        return Ok(AnalyzeResponse {
            job_id: "".to_string(), // This will be filled later, or handled differently if no data.
            asset_class: payload.asset_class.to_string(),
            data: Vec::new(),
        });
    }

    let aggregated_df = df
        .lazy()
        .group_by([col("ticker")])
        .agg([
            col("time").min().alias("start_time"),
            col("time").max().alias("end_time"),
            // Conditional aggregation based on asset class
            if payload.asset_class == AssetClass::Crypto {
                // Compare with enum variant
                col("last_price").count().alias("data_points")
            } else {
                col("mid_price").count().alias("data_points")
            },
            if payload.asset_class == AssetClass::Crypto {
                // Compare with enum variant
                col("last_price").mean().alias("avg_price")
            } else {
                col("mid_price").mean().alias("avg_mid")
            },
        ])
        .collect()?;

    let json_data = dataframe_to_json_value(&aggregated_df)?;

    let data_vec: Vec<Value> = if let Some(arr) = json_data.as_array() {
        arr.to_vec()
    } else {
        vec![json_data]
    };

    Ok(AnalyzeResponse {
        job_id: "".to_string(),
        asset_class: payload.asset_class.to_string(), // Convert AssetClass back to String
        data: data_vec,
    })
}
