//! `POST /embeddings` resource, mirroring `resources/embeddings.py`.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::RequestOptions;
use crate::types::embeddings::{EmbeddingRequest, EmbeddingResponse};

/// The embeddings resource.
#[derive(Debug, Clone)]
pub struct Embeddings {
    client: Client,
}

impl Embeddings {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create embeddings for the given input.
    pub async fn create(
        &self,
        request: EmbeddingRequest,
    ) -> Result<EmbeddingResponse, OpenAIError> {
        let body = serde_json::to_value(&request)?;
        self.client
            .execute(Method::POST, "/embeddings", RequestOptions::json(body))
            .await
    }
}
