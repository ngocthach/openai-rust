//! `POST /moderations` resource, mirroring `resources/moderations.py`.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::RequestOptions;
use crate::types::moderations::{ModerationRequest, ModerationResponse};

/// The moderations resource.
#[derive(Debug, Clone)]
pub struct Moderations {
    client: Client,
}

impl Moderations {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Classify whether the input violates the provider's content policy.
    pub async fn create(
        &self,
        request: ModerationRequest,
    ) -> Result<ModerationResponse, OpenAIError> {
        let body = serde_json::to_value(&request)?;
        self.client
            .execute(Method::POST, "/moderations", RequestOptions::json(body))
            .await
    }
}
