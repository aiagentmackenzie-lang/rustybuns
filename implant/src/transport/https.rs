use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use super::{Transport, TransportError};

pub struct HttpsTransport {
    base_url: String,
    client: reqwest::Client,
}

impl HttpsTransport {
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to create HTTP client");
        Self {
            base_url: base_url.to_string(),
            client,
        }
    }
}

#[async_trait]
impl Transport for HttpsTransport {
    async fn send<T: Serialize + Send + Sync>(&self, endpoint: &str, body: &T) -> Result<(), TransportError> {
        let url = format!("{}{}", self.base_url, endpoint);
        self.client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
        Ok(())
    }

    async fn recv<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T, TransportError> {
        let url = format!("{}{}", self.base_url, endpoint);
        let resp = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
        if !resp.status().is_success() {
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(TransportError::RequestFailed("not found".to_string()));
            }
            return Err(TransportError::RequestFailed(format!("status: {}", resp.status())));
        }
        resp.json::<T>()
            .await
            .map_err(|e| TransportError::ParseFailed(e.to_string()))
    }
}
