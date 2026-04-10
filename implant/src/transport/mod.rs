pub mod https;

pub use https::HttpsTransport;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send<T: Serialize + Send + Sync>(&self, endpoint: &str, body: &T) -> Result<(), TransportError>;
    async fn recv<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T, TransportError>;
}

#[derive(Debug)]
pub enum TransportError {
    RequestFailed(String),
    ParseFailed(String),
    ConnectionFailed(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::RequestFailed(s) => write!(f, "request failed: {}", s),
            TransportError::ParseFailed(s) => write!(f, "parse failed: {}", s),
            TransportError::ConnectionFailed(s) => write!(f, "connection failed: {}", s),
        }
    }
}

impl std::error::Error for TransportError {}
