//! `POST /chat/completions` resource, mirroring
//! `resources/chat/completions/completions.py`.

use std::collections::HashMap;

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::pagination::List;
use crate::request::{api_path, RequestOptions};
use crate::streaming::EventStream;
use crate::types::chat::{
    ChatCompletion, ChatCompletionChunk, ChatCompletionDeleted, ChatCompletionListParams,
    ChatCompletionMessagesListParams, ChatCompletionRequest, ChatCompletionStoreMessage,
};

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

    /// Retrieve a stored chat completion by id.
    pub async fn retrieve(&self, completion_id: &str) -> Result<ChatCompletion, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &api_path!("/chat/completions/{}", completion_id),
                RequestOptions::default(),
            )
            .await
    }

    /// Replace the metadata of a stored chat completion (POST, not PATCH).
    pub async fn update(
        &self,
        completion_id: &str,
        metadata: HashMap<String, String>,
    ) -> Result<ChatCompletion, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &api_path!("/chat/completions/{}", completion_id),
                RequestOptions::json(serde_json::json!({ "metadata": metadata })),
            )
            .await
    }

    /// List stored chat completions (single page).
    pub async fn list(
        &self,
        params: Option<ChatCompletionListParams>,
    ) -> Result<List<ChatCompletion>, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .execute(
                Method::GET,
                "/chat/completions",
                RequestOptions::query(query),
            )
            .await
    }

    /// List every stored chat completion, following pagination cursors.
    pub async fn list_all(
        &self,
        params: Option<ChatCompletionListParams>,
    ) -> Result<Vec<ChatCompletion>, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client.paginate_all("/chat/completions", query).await
    }

    /// Delete a stored chat completion.
    pub async fn delete(&self, completion_id: &str) -> Result<ChatCompletionDeleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &api_path!("/chat/completions/{}", completion_id),
                RequestOptions::default(),
            )
            .await
    }

    /// List the messages of a stored chat completion (single page).
    pub async fn messages(
        &self,
        completion_id: &str,
        params: Option<ChatCompletionMessagesListParams>,
    ) -> Result<List<ChatCompletionStoreMessage>, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .execute(
                Method::GET,
                &api_path!("/chat/completions/{}/messages", completion_id),
                RequestOptions::query(query),
            )
            .await
    }

    /// List every message of a stored chat completion, following cursors.
    pub async fn messages_all(
        &self,
        completion_id: &str,
        params: Option<ChatCompletionMessagesListParams>,
    ) -> Result<Vec<ChatCompletionStoreMessage>, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .paginate_all(
                &api_path!("/chat/completions/{}/messages", completion_id),
                query,
            )
            .await
    }
}
