//! Batch types, mirroring `openai-python/src/openai/types/batch.py` and
//! `batch_create_params.py` / `batch_list_params.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::pagination::HasId;

/// The processing status of a [`Batch`].
///
/// Unrecognized values deserialize to [`BatchStatus::Unknown`] so newer
/// server-side statuses do not break older clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Validating,
    Failed,
    InProgress,
    Finalizing,
    Completed,
    Expired,
    Cancelling,
    Cancelled,
    /// A status not recognized by this client version.
    #[serde(other)]
    Unknown,
}

/// A single error entry from a batch's `errors` list.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct BatchError {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub line: Option<i64>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub param: Option<String>,
}

/// The `errors` object on a [`Batch`].
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct BatchErrors {
    #[serde(default)]
    pub data: Vec<BatchError>,
    #[serde(default)]
    pub object: Option<String>,
}

/// Counts of the requests within a batch, split by outcome.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct BatchRequestCounts {
    #[serde(default)]
    pub completed: i64,
    #[serde(default)]
    pub failed: i64,
    #[serde(default)]
    pub total: i64,
}

/// A batch job over an uploaded input file.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Batch {
    pub id: String,
    #[serde(default)]
    pub completion_window: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub input_file_id: String,
    #[serde(default)]
    pub object: String,
    pub status: BatchStatus,
    #[serde(default)]
    pub cancelled_at: Option<i64>,
    #[serde(default)]
    pub cancelling_at: Option<i64>,
    #[serde(default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub expired_at: Option<i64>,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub failed_at: Option<i64>,
    #[serde(default)]
    pub finalizing_at: Option<i64>,
    #[serde(default)]
    pub in_progress_at: Option<i64>,
    #[serde(default)]
    pub error_file_id: Option<String>,
    #[serde(default)]
    pub output_file_id: Option<String>,
    #[serde(default)]
    pub errors: Option<BatchErrors>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub request_counts: Option<BatchRequestCounts>,
    #[serde(default)]
    pub usage: Option<serde_json::Value>,
}

impl HasId for Batch {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// Parameters for `POST /batches`.
#[derive(Debug, Clone, Serialize)]
pub struct BatchCreateParams {
    /// The time window for completion. Currently only `"24h"` is supported.
    pub completion_window: String,
    /// The endpoint the batched requests target, e.g. `/v1/chat/completions`.
    pub endpoint: String,
    /// The id of an uploaded file containing the batched requests (JSONL).
    pub input_file_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Expiration policy for the batch output/error files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_expires_after: Option<crate::types::common::ExpiresAfter>,
}

impl BatchCreateParams {
    pub fn new(
        input_file_id: impl Into<String>,
        endpoint: impl Into<String>,
        completion_window: impl Into<String>,
    ) -> Self {
        Self {
            completion_window: completion_window.into(),
            endpoint: endpoint.into(),
            input_file_id: input_file_id.into(),
            metadata: None,
            output_expires_after: None,
        }
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Query parameters for `GET /batches`.
#[derive(Debug, Clone, Default)]
pub struct BatchListParams {
    /// A cursor: return items after this batch id.
    pub after: Option<String>,
    /// Page size, 1-100 (server default 20).
    pub limit: Option<u32>,
}

impl BatchListParams {
    pub fn after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Build the `(key, value)` query pairs for the list request.
    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        crate::pagination::cursor_query(self.after.as_deref(), None, self.limit, None)
    }
}
