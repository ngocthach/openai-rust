//! `/uploads` resource, mirroring `resources/uploads/`: create an upload, add
//! byte parts (multipart), then complete or cancel it.

use reqwest::Method;
use serde_json::json;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::{api_path, MultipartField, RequestOptions};
use crate::types::uploads::{Upload, UploadCreateParams, UploadPart};

/// The uploads resource.
#[derive(Debug, Clone)]
pub struct Uploads {
    client: Client,
}

impl Uploads {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a resumable upload. Parts are added with [`Uploads::add_part`]
    /// and finalized with [`Uploads::complete`].
    pub async fn create(&self, params: UploadCreateParams) -> Result<Upload, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(Method::POST, "/uploads", RequestOptions::json(body))
            .await
    }

    /// Add a chunk of bytes to an upload as a multipart `data` field.
    pub async fn add_part(
        &self,
        upload_id: &str,
        data: Vec<u8>,
    ) -> Result<UploadPart, OpenAIError> {
        let fields = vec![MultipartField::File {
            name: "data".into(),
            filename: "part".into(),
            bytes: data,
            content_type: None,
        }];
        self.client
            .execute(
                Method::POST,
                &api_path!("/uploads/{}/parts", upload_id),
                RequestOptions::multipart(fields),
            )
            .await
    }

    /// Complete an upload by ordering its parts, optionally verifying the whole
    /// file against an `md5` checksum.
    pub async fn complete(
        &self,
        upload_id: &str,
        part_ids: Vec<String>,
        md5: Option<&str>,
    ) -> Result<Upload, OpenAIError> {
        let mut body = json!({ "part_ids": part_ids });
        if let Some(md5) = md5 {
            body["md5"] = json!(md5);
        }
        self.client
            .execute(
                Method::POST,
                &api_path!("/uploads/{}/complete", upload_id),
                RequestOptions::json(body),
            )
            .await
    }

    /// Cancel an upload, invalidating it and its parts.
    pub async fn cancel(&self, upload_id: &str) -> Result<Upload, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &api_path!("/uploads/{}/cancel", upload_id),
                RequestOptions::default(),
            )
            .await
    }
}
