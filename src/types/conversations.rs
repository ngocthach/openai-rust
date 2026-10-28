//! Conversations API types, mirroring `openai-python/src/openai/types/conversations/`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::pagination::{cursor_query, HasId, List};
use crate::types::responses::{InputItem, ResponseIncludable};

/// A conversation: a durable container of items shared across responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Conversation {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub object: String,
}

/// Result of deleting a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ConversationDeleted {
    pub id: String,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub object: String,
}

/// An item in a conversation. Upstream models this as a union of ~30 message,
/// tool-call and content shapes; the stable envelope (`id`, `type`) is typed
/// and every other field is kept in `rest` so nothing is dropped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, rename = "type")]
    pub item_type: String,
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

impl HasId for ConversationItem {
    fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

/// A page of conversation items. `last_id` (not the last item's id) is the
/// cursor for the next page.
pub type ConversationItemList = List<ConversationItem>;

/// Request body for `POST /conversations`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ConversationCreateParams {
    /// Initial items for the conversation (at most 20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<InputItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl ConversationCreateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: Vec<InputItem>) -> Self {
        self.items = Some(items);
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Request body for `POST /conversations/{id}`.
#[derive(Debug, Clone, Serialize)]
pub struct ConversationUpdateParams {
    pub metadata: HashMap<String, String>,
}

impl ConversationUpdateParams {
    pub fn new(metadata: HashMap<String, String>) -> Self {
        Self { metadata }
    }
}

/// Parameters for `POST /conversations/{id}/items`: the items go in the body,
/// `include` in the query string.
#[derive(Debug, Clone, Serialize)]
pub struct ConversationItemsCreateParams {
    /// Items to add (at most 20 per call).
    pub items: Vec<InputItem>,
    #[serde(skip)]
    pub include: Option<Vec<ResponseIncludable>>,
}

impl ConversationItemsCreateParams {
    pub fn new(items: Vec<InputItem>) -> Self {
        Self {
            items,
            include: None,
        }
    }

    /// Additional fields to include in the returned items.
    pub fn include(mut self, include: Vec<ResponseIncludable>) -> Self {
        self.include = Some(include);
        self
    }
}

/// Repeated `include[]` query pairs.
pub(crate) fn include_query(include: Option<&[ResponseIncludable]>) -> Vec<(String, String)> {
    include
        .unwrap_or_default()
        .iter()
        .map(|item| ("include[]".to_string(), item.as_str().to_string()))
        .collect()
}

/// Query parameters for `GET /conversations/{id}/items`.
#[derive(Debug, Clone, Default)]
pub struct ConversationItemsListParams {
    pub after: Option<String>,
    pub limit: Option<u32>,
    pub order: Option<String>,
    pub include: Option<Vec<ResponseIncludable>>,
}

impl ConversationItemsListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sort order: `"asc"` or `"desc"`.
    pub fn order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    pub fn include(mut self, include: Vec<ResponseIncludable>) -> Self {
        self.include = Some(include);
        self
    }

    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        let mut query = cursor_query(
            self.after.as_deref(),
            None,
            self.limit,
            self.order.as_deref(),
        );
        query.extend(include_query(self.include.as_deref()));
        query
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_keeps_unmodeled_fields_round_trip() {
        let raw = serde_json::json!({
            "id": "msg_1", "type": "message", "role": "user",
            "content": [{"type": "input_text", "text": "hi"}]
        });
        let item: ConversationItem = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(item.id(), Some("msg_1"));
        assert_eq!(item.item_type, "message");
        assert_eq!(serde_json::to_value(&item).unwrap(), raw);
    }
}
