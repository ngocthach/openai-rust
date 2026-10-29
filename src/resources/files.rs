//! `/files` resource, mirroring `resources/files.py`: list, retrieve,
//! delete, content download, and multipart upload.

use bytes::Bytes;
use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::pagination::List;
use crate::request::{api_path, MultipartField, RequestOptions};
use crate::types::files::{FileObject, FileUpload};
use crate::types::models::Deleted;

/// The files resource.
#[derive(Debug, Clone)]
pub struct Files {
    client: Client,
}

impl Files {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Upload a file for the given purpose (e.g. `"fine-tune"`, `"batch"`,
    /// `"assistants"`, `"user_data"`).
    pub async fn create(&self, file: FileUpload, purpose: &str) -> Result<FileObject, OpenAIError> {
        let fields = vec![
            MultipartField::File {
                name: "file".into(),
                filename: file.filename,
                bytes: file.bytes,
                content_type: file.content_type,
            },
            MultipartField::Text {
                name: "purpose".into(),
                value: purpose.to_string(),
            },
        ];
        self.client
            .execute(Method::POST, "/files", RequestOptions::multipart(fields))
            .await
    }

    /// List files, optionally filtered by purpose. Fetches a single page.
    pub async fn list(&self, purpose: Option<&str>) -> Result<List<FileObject>, OpenAIError> {
        let query = purpose
            .map(|p| vec![("purpose".to_string(), p.to_string())])
            .unwrap_or_default();
        self.client
            .execute(Method::GET, "/files", RequestOptions::query(query))
            .await
    }

    /// List all files, following pagination cursors.
    pub async fn list_all(&self, purpose: Option<&str>) -> Result<Vec<FileObject>, OpenAIError> {
        let query = purpose
            .map(|p| vec![("purpose".to_string(), p.to_string())])
            .unwrap_or_default();
        self.client.paginate_all("/files", query).await
    }

    /// Retrieve file metadata.
    pub async fn retrieve(&self, file_id: &str) -> Result<FileObject, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &api_path!("/files/{}", file_id),
                RequestOptions::default(),
            )
            .await
    }

    /// Delete a file.
    pub async fn delete(&self, file_id: &str) -> Result<Deleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &api_path!("/files/{}", file_id),
                RequestOptions::default(),
            )
            .await
    }

    /// Download the file's raw content.
    pub async fn content(&self, file_id: &str) -> Result<Bytes, OpenAIError> {
        self.client
            .execute_bytes(
                Method::GET,
                &api_path!("/files/{}/content", file_id),
                RequestOptions::default(),
            )
            .await
    }
}
