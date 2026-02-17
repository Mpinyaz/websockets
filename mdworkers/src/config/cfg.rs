use anyhow::Result;
use std::error::Error;
use std::fmt;

#[derive(Clone)]
pub struct Config {
    pub data_url: String,
    pub data_api_key: String,
    pub stream_host: String,
    pub stream_port: u16,
    pub stream_username: String,
    pub stream_password: String,
    pub feed_stream: String,
    pub subscribe_stream: String,
}

pub enum ConfigError {
    NotFound(String),
    ParseError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NotFound(field) => write!(f, "{} must be set", field),
            Self::ParseError(field) => write!(f, "Invalid {}", field),
        }
    }
}

impl fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error for ConfigError {}

impl Config {
    pub fn load_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let get_env =
            |key: &str| std::env::var(key).map_err(|_| ConfigError::NotFound(key.to_string()));

        Ok(Config {
            data_url: get_env("TIINGO_WS_URL")?,
            data_api_key: get_env("TIINGO_API_KEY")?,
            stream_host: get_env("RABBITMQ_ADVERTISED_HOST")?,
            stream_port: get_env("RABBITMQ_STREAM_PORT")?
                .parse()
                .map_err(|e| ConfigError::ParseError(format!("RABBITMQ_STREAM_PORT: {e}")))?,
            stream_username: get_env("RABBITMQ_DEFAULT_USER")?,
            stream_password: get_env("RABBITMQ_DEFAULT_PASS")?,
            feed_stream: get_env("MDWS_FEED_STREAM")?,
            subscribe_stream: get_env("MDWS_SUBSCRIBE_STREAM")?,
        })
    }
}
