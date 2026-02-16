use anyhow::Result;
use rabbitmq_stream_client::{
    Consumer,
    types::{ByteCapacity, ResponseCode, StreamCreator},
};
use rabbitmq_stream_client::{Environment, Producer};
use rabbitmq_stream_client::{NoDedup, error::StreamCreateError};
use tokio::sync::OnceCell;
use tracing::info;

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

    let mkt_feed_producer = environment.producer().build(&config.feed_stream).await?;

    let mkt_sub_producer = environment
        .producer()
        .build(&config.subscribe_stream)
        .await?;

    info!(
        "Stream producers initialized: feed={}, subs={}",
        &config.feed_stream, &config.subscribe_stream
    );

    Ok(StreamEntities {
        mkt_feed_producer,
        mkt_sub_producer,
    })
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
