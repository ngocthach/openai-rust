//! Vector store types, mirroring
//! `openai-python/src/openai/types/vector_store.py`,
//! `types/vector_stores/vector_store_file.py`,
//! `vector_store_file_batch.py`, and the create/update/search params.
//!
//! Deep polymorphic fields (`filters`, `ranking_options`, `chunking_strategy`,
//! `attributes`, `metadata`) are kept as [`serde_json::Value`]: their shapes are
//! open-ended and best passed through untyped.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pagination::HasId;

// Re-use the shared `{ id, deleted, object }` shape for both delete responses.
pub use crate::types::models::Deleted as VectorStoreDeleted;
pub use crate::types::models::Deleted as VectorStoreFileDeleted;

/// File-processing counts for a vector store (or a file batch).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FileCounts {
    #[serde(default)]
    pub in_progress: i32,
    #[serde(default)]
    pub completed: i32,
    #[serde(default)]
    pub failed: i32,
    #[serde(default)]
    pub cancelled: i32,
    #[serde(default)]
    pub total: i32,
}

/// The expiration policy for a vector store. Used in both requests and
/// responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiresAfter {
    /// Anchor timestamp after which the policy applies. Only `last_active_at`
    /// is currently supported.
    pub anchor: String,
    /// Days after the anchor time that the vector store expires.
    pub days: i32,
}

impl ExpiresAfter {
    /// Expire `days` after the store was last active.
    pub fn last_active_at(days: i32) -> Self {
        Self {
            anchor: "last_active_at".to_string(),
            days,
        }
    }
}

/// Lifecycle status of a vector store.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum VectorStoreStatus {
    Expired,
    InProgress,
    Completed,
    /// A status not known to this client version.
    #[default]
    #[serde(other)]
    Unknown,
}

/// A vector store: a collection of processed files usable by `file_search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorStore {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub status: VectorStoreStatus,
    #[serde(default)]
    pub usage_bytes: i64,
    #[serde(default)]
    pub file_counts: FileCounts,
    #[serde(default)]
    pub last_active_at: Option<i64>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub expires_after: Option<ExpiresAfter>,
    #[serde(default)]
    pub expires_at: Option<i64>,
}

impl HasId for VectorStore {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// Processing status of a vector store file or file batch.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum VectorStoreFileStatus {
    InProgress,
    Completed,
    Cancelled,
    Failed,
    /// A status not known to this client version.
    #[default]
    #[serde(other)]
    Unknown,
}

/// The last error associated with a vector store file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorStoreFileLastError {
    /// One of `server_error`, `unsupported_file`, or `invalid_file`.
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
}

/// A file attached to a vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorStoreFile {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub vector_store_id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub status: VectorStoreFileStatus,
    #[serde(default)]
    pub usage_bytes: i64,
    #[serde(default)]
    pub last_error: Option<VectorStoreFileLastError>,
    #[serde(default)]
    pub attributes: Option<Value>,
    /// The chunking strategy used to chunk the file (open-ended shape).
    #[serde(default)]
    pub chunking_strategy: Option<Value>,
}

impl HasId for VectorStoreFile {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// A batch of files attached to a vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorStoreFileBatch {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub vector_store_id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub status: VectorStoreFileStatus,
    #[serde(default)]
    pub file_counts: FileCounts,
}

impl HasId for VectorStoreFileBatch {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// One text chunk in a search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorStoreSearchContent {
    #[serde(default)]
    pub text: String,
    /// The type of content, e.g. `"text"`.
    #[serde(rename = "type", default)]
    pub kind: String,
}

/// One result from `POST /vector_stores/{id}/search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorStoreSearchResponse {
    #[serde(default)]
    pub file_id: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub attributes: Option<Value>,
    #[serde(default)]
    pub content: Vec<VectorStoreSearchContent>,
}

/// One item of a vector store file's parsed content
/// (`GET .../files/{file_id}/content`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FileContentResponse {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
}

// ---------------------------------------------------------------------------
// Request params
// ---------------------------------------------------------------------------

/// Request body for `POST /vector_stores`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct VectorStoreCreateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<ExpiresAfter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<Value>,
}

impl VectorStoreCreateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn file_ids(mut self, file_ids: Vec<String>) -> Self {
        self.file_ids = Some(file_ids);
        self
    }

    pub fn metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn expires_after(mut self, expires_after: ExpiresAfter) -> Self {
        self.expires_after = Some(expires_after);
        self
    }

    pub fn chunking_strategy(mut self, chunking_strategy: Value) -> Self {
        self.chunking_strategy = Some(chunking_strategy);
        self
    }
}

/// Request body for `POST /vector_stores/{id}` (update).
#[derive(Debug, Clone, Default, Serialize)]
pub struct VectorStoreUpdateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<ExpiresAfter>,
}

impl VectorStoreUpdateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn expires_after(mut self, expires_after: ExpiresAfter) -> Self {
        self.expires_after = Some(expires_after);
        self
    }
}

/// Query string for a vector store list request.
#[derive(Debug, Clone, Default)]
pub struct VectorStoreListParams {
    pub after: Option<String>,
    pub before: Option<String>,
    pub limit: Option<u32>,
    /// `"asc"` or `"desc"`.
    pub order: Option<String>,
}

impl VectorStoreListParams {
    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        crate::pagination::cursor_query(
            self.after.as_deref(),
            self.before.as_deref(),
            self.limit,
            self.order.as_deref(),
        )
    }
}

/// A search query: a single string or a list of strings.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SearchQuery {
    Text(String),
    Multiple(Vec<String>),
}

impl From<&str> for SearchQuery {
    fn from(value: &str) -> Self {
        SearchQuery::Text(value.to_string())
    }
}

impl From<String> for SearchQuery {
    fn from(value: String) -> Self {
        SearchQuery::Text(value)
    }
}

impl From<Vec<String>> for SearchQuery {
    fn from(value: Vec<String>) -> Self {
        SearchQuery::Multiple(value)
    }
}

/// Request body for `POST /vector_stores/{id}/search`.
#[derive(Debug, Clone, Serialize)]
pub struct VectorStoreSearchParams {
    pub query: SearchQuery,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_options: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewrite_query: Option<bool>,
}

impl VectorStoreSearchParams {
    pub fn new(query: impl Into<SearchQuery>) -> Self {
        Self {
            query: query.into(),
            filters: None,
            max_num_results: None,
            ranking_options: None,
            rewrite_query: None,
        }
    }

    pub fn filters(mut self, filters: Value) -> Self {
        self.filters = Some(filters);
        self
    }

    pub fn max_num_results(mut self, max_num_results: u32) -> Self {
        self.max_num_results = Some(max_num_results);
        self
    }

    pub fn ranking_options(mut self, ranking_options: Value) -> Self {
        self.ranking_options = Some(ranking_options);
        self
    }

    pub fn rewrite_query(mut self, rewrite_query: bool) -> Self {
        self.rewrite_query = Some(rewrite_query);
        self
    }
}

/// Request body for `POST /vector_stores/{id}/files`.
#[derive(Debug, Clone, Serialize)]
pub struct VectorStoreFileCreateParams {
    pub file_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<Value>,
}

impl VectorStoreFileCreateParams {
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
            attributes: None,
            chunking_strategy: None,
        }
    }

    pub fn attributes(mut self, attributes: Value) -> Self {
        self.attributes = Some(attributes);
        self
    }

    pub fn chunking_strategy(mut self, chunking_strategy: Value) -> Self {
        self.chunking_strategy = Some(chunking_strategy);
        self
    }
}

/// Query string for a vector store file list request.
#[derive(Debug, Clone, Default)]
pub struct VectorStoreFileListParams {
    pub after: Option<String>,
    pub before: Option<String>,
    /// Filter by status: `in_progress`, `completed`, `failed`, `cancelled`.
    pub filter: Option<String>,
    pub limit: Option<u32>,
    /// `"asc"` or `"desc"`.
    pub order: Option<String>,
}

impl VectorStoreFileListParams {
    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        let mut query = crate::pagination::cursor_query(
            self.after.as_deref(),
            self.before.as_deref(),
            self.limit,
            self.order.as_deref(),
        );
        if let Some(filter) = &self.filter {
            query.push(("filter".to_string(), filter.clone()));
        }
        query
    }
}

/// Request body for `POST /vector_stores/{id}/file_batches`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct VectorStoreFileBatchCreateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<Value>,
}

impl VectorStoreFileBatchCreateParams {
    pub fn new(file_ids: Vec<String>) -> Self {
        Self {
            file_ids: Some(file_ids),
            attributes: None,
            chunking_strategy: None,
        }
    }

    pub fn attributes(mut self, attributes: Value) -> Self {
        self.attributes = Some(attributes);
        self
    }

    pub fn chunking_strategy(mut self, chunking_strategy: Value) -> Self {
        self.chunking_strategy = Some(chunking_strategy);
        self
    }
}
