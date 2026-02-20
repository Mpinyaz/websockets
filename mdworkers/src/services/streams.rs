use anyhow::Result;
use rabbitmq_stream_client::NoDedup;
use rabbitmq_stream_client::{error::StreamCreateError, Environment, Producer};
use rabbitmq_stream_client::{
    types::{ByteCapacity, ResponseCode, StreamCreator},
    Consumer,
};
use tokio::sync::OnceCell;
use tokio::time::sleep;
use tracing::{error, info};

use crate::config::cfg::Config;

pub struct StreamEntities {
    pub mkt_feed_producer: Producer<NoDedup>,
    pub mkt_sub_producer: Producer<NoDedup>,
}

static STREAM_ENTITIES: OnceCell<StreamEntities> = OnceCell::const_new();

pub async fn get_stream_entities() -> Result<&'static StreamEntities> {
    STREAM_ENTITIES.get_or_try_init(init_stream_entities).await
}

async fn init_stream_entities() -> Result<StreamEntities> {
    let config = Config::load_env()?;

    let environment = Environment::builder()
        .host(&config.stream_host)
        .port(config.stream_port)
        .username(&config.stream_username)
        .password(&config.stream_password)
        .build()
        .await?;

    // Market data stream
    let updates_env = environment
        .stream_creator()
        .max_length(ByteCapacity::GB(5))
        .max_age(std::time::Duration::from_secs(3600 * 24 * 7));
    init_stream(updates_env, &config.feed_stream).await?;

    let subscribe_env = environment
        .stream_creator()
        .max_length(ByteCapacity::GB(2))
        .max_age(std::time::Duration::from_secs(3600 * 24 * 3));
    init_stream(subscribe_env, &config.subscribe_stream).await?;

    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAY_SECS: u64 = 5;

    for attempt in 1..=MAX_RETRIES {
        info!("Creating producers, attempt {}/{}", attempt, MAX_RETRIES);

        let producers_result: Result<_, anyhow::Error> = async {
            let mkt_feed_producer = environment.producer().build(&config.feed_stream).await?;
            let mkt_sub_producer = environment
                .producer()
                .build(&config.subscribe_stream)
                .await?;
            Ok((mkt_feed_producer, mkt_sub_producer))
        }
        .await;

        match producers_result {
            Ok((mkt_feed_producer, mkt_sub_producer)) => {
                info!(
                    "Stream producers initialized: feed={}, subs={}",
                    &config.feed_stream, &config.subscribe_stream
                );
                return Ok(StreamEntities {
                    mkt_feed_producer,
                    mkt_sub_producer,
                });
            }
            Err(e) => {
                error!(
                    "Failed to create producers (attempt {}/{}): {}. Retrying in {}s...",
                    attempt, MAX_RETRIES, e, RETRY_DELAY_SECS
                );
                if attempt == MAX_RETRIES {
                    return Err(e);
                }
                sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
            }
        }
    }

    unreachable!();
}

pub async fn create_sub_consumer() -> Result<Consumer> {
    let config = Config::load_env()?;

    let environment = Environment::builder()
        .host(&config.stream_host)
        .port(config.stream_port)
        .username(&config.stream_username)
        .password(&config.stream_password)
        .build()
        .await?;

    let consumer = environment
        .consumer()
        .build(&config.subscribe_stream)
        .await?;

    info!(
        "Subscription consumer created for stream '{}'",
        &config.subscribe_stream
    );

    Ok(consumer)
}

async fn init_stream(builder: StreamCreator, stream_name: &String) -> Result<()> {
    match builder.create(stream_name).await {
        Ok(_) => info!("Stream '{}' created", stream_name),
        Err(StreamCreateError::Create {
            status: ResponseCode::StreamAlreadyExists | ResponseCode::PreconditionFailed,
            ..
        }) => {
            info!("Stream '{}' already exists", stream_name);
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}
