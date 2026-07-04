//! Resumable upload types, mirroring `openai-python/src/openai/types/upload.py`
//! and `types/uploads/upload_part.py`.

use serde::{Deserialize, Serialize};

use crate::pagination::HasId;
use crate::types::files::FileObject;

/// Default chunk size for splitting a large upload into parts (64 MiB).
pub const DEFAULT_PART_SIZE: usize = 64 * 1024 * 1024;

/// The status of an [`Upload`].
///
/// Unrecognized values deserialize to [`UploadStatus::Unknown`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    Pending,
    Completed,
    Cancelled,
    Expired,
    /// A status not recognized by this client version.
    #[serde(other)]
    Unknown,
}

/// A resumable upload: created, filled with parts, then completed.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Upload {
    pub id: String,
    #[serde(default)]
    pub bytes: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub expires_at: i64,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub purpose: String,
    pub status: UploadStatus,
    /// The ready [`FileObject`], present once the upload is completed.
    #[serde(default)]
    pub file: Option<FileObject>,
}

impl HasId for Upload {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// A single uploaded part of an [`Upload`].
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct UploadPart {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub upload_id: String,
    #[serde(default)]
    pub object: String,
}

impl HasId for UploadPart {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// Parameters for `POST /uploads`.
#[derive(Debug, Clone, Serialize)]
pub struct UploadCreateParams {
    /// The total number of bytes that will be uploaded across all parts.
    pub bytes: i64,
    /// The name of the file to be uploaded.
    pub filename: String,
    /// The MIME type of the file, e.g. `application/jsonl`.
    pub mime_type: String,
    /// The intended purpose, e.g. `assistants`, `batch`, `fine-tune`, `vision`,
    /// `user_data`, or `evals`.
    pub purpose: String,
}

impl UploadCreateParams {
    pub fn new(
        filename: impl Into<String>,
        bytes: i64,
        mime_type: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Self {
        Self {
            bytes,
            filename: filename.into(),
            mime_type: mime_type.into(),
            purpose: purpose.into(),
        }
    }
}
