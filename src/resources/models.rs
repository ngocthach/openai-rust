//! `/models` resource, mirroring `resources/models.py`.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::pagination::List;
use crate::request::RequestOptions;
use crate::types::models::{Deleted, Model};

/// The models resource.
#[derive(Debug, Clone)]
pub struct Models {
    client: Client,
}

impl Models {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// List the currently available models.
    pub async fn list(&self) -> Result<List<Model>, OpenAIError> {
        self.client
            .execute(Method::GET, "/models", RequestOptions::default())
            .await
    }

    /// Retrieve a model by id.
    pub async fn retrieve(&self, model: &str) -> Result<Model, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("/models/{model}"),
                RequestOptions::default(),
            )
            .await
    }

    /// Delete a fine-tuned model. Requires owner permissions.
    pub async fn delete(&self, model: &str) -> Result<Deleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &format!("/models/{model}"),
                RequestOptions::default(),
            )
            .await
    }
}
