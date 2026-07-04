//! Legacy `POST /completions` resource, mirroring `resources/completions.py`.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::RequestOptions;
use crate::streaming::EventStream;
use crate::types::completions::{Completion, CompletionRequest};

/// The legacy completions resource.
#[derive(Debug, Clone)]
pub struct Completions {
    client: Client,
}

impl Completions {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a completion and wait for the full response.
    pub async fn create(&self, mut request: CompletionRequest) -> Result<Completion, OpenAIError> {
        request.stream = None;
        let body = serde_json::to_value(&request)?;
        self.client
            .execute(Method::POST, "/completions", RequestOptions::json(body))
            .await
    }

    /// Create a completion and stream partial results.
    pub async fn create_stream(
        &self,
        mut request: CompletionRequest,
    ) -> Result<EventStream<Completion>, OpenAIError> {
        request.stream = Some(true);
        let body = serde_json::to_value(&request)?;
        let response = self
            .client
            .execute_raw(Method::POST, "/completions", RequestOptions::json(body))
            .await?;
        Ok(EventStream::new(response))
    }
}
