//! `/conversations` resource, mirroring
//! `openai-python/src/openai/resources/conversations/`: conversation CRUD plus
//! the `.items(id)` sub-resource.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::{api_path, RequestOptions};
use crate::types::conversations::{
    include_query, Conversation, ConversationCreateParams, ConversationDeleted, ConversationItem,
    ConversationItemList, ConversationItemsCreateParams, ConversationItemsListParams,
    ConversationUpdateParams,
};
use crate::types::responses::ResponseIncludable;

/// The conversations resource.
#[derive(Debug, Clone)]
pub struct Conversations {
    client: Client,
}

impl Conversations {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a conversation, optionally seeded with items and metadata.
    pub async fn create(
        &self,
        params: ConversationCreateParams,
    ) -> Result<Conversation, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(Method::POST, "/conversations", RequestOptions::json(body))
            .await
    }

    /// Retrieve a conversation by id.
    pub async fn retrieve(&self, conversation_id: &str) -> Result<Conversation, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &api_path!("/conversations/{}", conversation_id),
                RequestOptions::default(),
            )
            .await
    }

    /// Update a conversation's metadata (POST, not PATCH).
    pub async fn update(
        &self,
        conversation_id: &str,
        params: ConversationUpdateParams,
    ) -> Result<Conversation, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                &api_path!("/conversations/{}", conversation_id),
                RequestOptions::json(body),
            )
            .await
    }

    /// Delete a conversation.
    pub async fn delete(&self, conversation_id: &str) -> Result<ConversationDeleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &api_path!("/conversations/{}", conversation_id),
                RequestOptions::default(),
            )
            .await
    }

    /// The items sub-resource for `conversation_id`.
    pub fn items(&self, conversation_id: impl Into<String>) -> ConversationItems {
        ConversationItems {
            client: self.client.clone(),
            conversation_id: conversation_id.into(),
        }
    }
}

/// Items of one conversation: `/conversations/{id}/items`.
#[derive(Debug, Clone)]
pub struct ConversationItems {
    client: Client,
    conversation_id: String,
}

impl ConversationItems {
    /// Add items to the conversation (at most 20 per call).
    pub async fn create(
        &self,
        params: ConversationItemsCreateParams,
    ) -> Result<ConversationItemList, OpenAIError> {
        let mut options = RequestOptions::json(serde_json::to_value(&params)?);
        options.query = include_query(params.include.as_deref());
        self.client
            .execute(
                Method::POST,
                &api_path!("/conversations/{}/items", self.conversation_id),
                options,
            )
            .await
    }

    /// Retrieve one item.
    pub async fn retrieve(
        &self,
        item_id: &str,
        include: Option<Vec<ResponseIncludable>>,
    ) -> Result<ConversationItem, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &api_path!("/conversations/{}/items/{}", self.conversation_id, item_id),
                RequestOptions::query(include_query(include.as_deref())),
            )
            .await
    }

    /// Fetch a single page of items.
    pub async fn list(
        &self,
        params: Option<ConversationItemsListParams>,
    ) -> Result<ConversationItemList, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .execute(
                Method::GET,
                &api_path!("/conversations/{}/items", self.conversation_id),
                RequestOptions::query(query),
            )
            .await
    }

    /// Fetch every item, following the page envelope's `last_id` cursor.
    pub async fn list_all(
        &self,
        params: Option<ConversationItemsListParams>,
    ) -> Result<Vec<ConversationItem>, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .paginate_all_by_last_id(
                &api_path!("/conversations/{}/items", self.conversation_id),
                query,
            )
            .await
    }

    /// Delete an item; returns the parent conversation.
    pub async fn delete(&self, item_id: &str) -> Result<Conversation, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &api_path!("/conversations/{}/items/{}", self.conversation_id, item_id),
                RequestOptions::default(),
            )
            .await
    }
}
