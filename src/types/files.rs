//! File types, mirroring `openai-python/src/openai/types/file_object.py`.

use serde::{Deserialize, Serialize};

use crate::pagination::HasId;

/// A file stored with the provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FileObject {
    pub id: String,
    #[serde(default)]
    pub bytes: Option<u64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub status_details: Option<String>,
}

impl HasId for FileObject {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// A file to upload: raw bytes plus a filename.
#[derive(Debug, Clone)]
pub struct FileUpload {
    pub filename: String,
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

impl FileUpload {
    pub fn new(filename: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            filename: filename.into(),
            bytes,
            content_type: None,
        }
    }

    /// Read the upload from disk, using the file name from the path.
    pub async fn from_path(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());
        let bytes = tokio::fs::read(path).await?;
        Ok(Self::new(filename, bytes))
    }
}
