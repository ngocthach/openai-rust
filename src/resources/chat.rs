//! `POST /chat/completions` resource, mirroring
//! `resources/chat/completions/completions.py`.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::RequestOptions;
use crate::streaming::EventStream;
use crate::types::chat::{ChatCompletion, ChatCompletionChunk, ChatCompletionRequest};

/// Accessor for chat endpoints: `client.chat().completions()`.
#[derive(Debug, Clone)]
pub struct Chat {
    client: Client,
}

impl Chat {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn completions(&self) -> ChatCompletions {
        ChatCompletions {
            client: self.client.clone(),
        }
    }
}

/// The chat completions resource.
#[derive(Debug, Clone)]
pub struct ChatCompletions {
    client: Client,
}

impl ChatCompletions {
    /// Create a chat completion and wait for the full response.
    pub async fn create(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<ChatCompletion, OpenAIError> {
        request.stream = None;
        let body = serde_json::to_value(&request)?;
        self.client
            .execute(
                Method::POST,
                "/chat/completions",
                RequestOptions::json(body),
            )
            .await
    }

    /// Create a chat completion and stream chunks as they are generated.
    pub async fn create_stream(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<EventStream<ChatCompletionChunk>, OpenAIError> {
        request.stream = Some(true);
        let body = serde_json::to_value(&request)?;
        let response = self
            .client
            .execute_raw(
                Method::POST,
                "/chat/completions",
                RequestOptions::json(body),
            )
            .await?;
        Ok(EventStream::new(response))
    }
}
