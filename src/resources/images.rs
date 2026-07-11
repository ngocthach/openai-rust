//! `POST /images/generations` resource, mirroring `resources/images.py`.
//! (Edits and variations are out of scope for v0.1.)

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::RequestOptions;
use crate::types::images::{ImageGenerationRequest, ImagesResponse};

/// The images resource.
#[derive(Debug, Clone)]
pub struct Images {
    client: Client,
}

impl Images {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Generate images from a text prompt.
    pub async fn generate(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImagesResponse, OpenAIError> {
        let body = serde_json::to_value(&request)?;
        self.client
            .execute(
                Method::POST,
                "/images/generations",
                RequestOptions::json(body),
            )
            .await
    }
}
