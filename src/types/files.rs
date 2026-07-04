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

    /// Read the upload from disk, using the file name from the path and
    /// inferring the content type from the extension (like the Python SDK's
    /// `mimetypes` guess).
    pub async fn from_path(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());
        let bytes = tokio::fs::read(path).await?;
        let mut upload = Self::new(filename, bytes);
        upload.content_type = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(mime_for_extension)
            .map(str::to_owned);
        Ok(upload)
    }
}

/// Content types for extensions commonly uploaded to LLM APIs.
fn mime_for_extension(ext: &str) -> Option<&'static str> {
    Some(match ext.to_ascii_lowercase().as_str() {
        "mp3" | "mpga" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" | "mp4" => "audio/mp4",
        "flac" => "audio/flac",
        "ogg" | "oga" => "audio/ogg",
        "webm" => "audio/webm",
        "json" => "application/json",
        "jsonl" => "application/jsonl",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => return None,
    })
}
