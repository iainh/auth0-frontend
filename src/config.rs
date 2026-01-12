use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug)]
pub struct Config {
    pub auth0_domain: String,
    pub auth0_client_id: String,
    pub auth0_client_secret: String,
    pub bind_addr: SocketAddr,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing environment variable: {0}")]
    Missing(&'static str),
    #[error("invalid bind address")]
    Addr(#[from] std::net::AddrParseError),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            auth0_domain: std::env::var("AUTH0_DOMAIN")
                .map_err(|_| ConfigError::Missing("AUTH0_DOMAIN"))?,
            auth0_client_id: std::env::var("AUTH0_CLIENT_ID")
                .map_err(|_| ConfigError::Missing("AUTH0_CLIENT_ID"))?,
            auth0_client_secret: std::env::var("AUTH0_CLIENT_SECRET")
                .map_err(|_| ConfigError::Missing("AUTH0_CLIENT_SECRET"))?,
            bind_addr: std::env::var("BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
                .parse()?,
        })
    }
}
