//! `/batches` resource, mirroring `resources/batches.py`: create, retrieve,
//! list (cursor-paginated), and cancel.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::pagination::List;
use crate::request::RequestOptions;
use crate::types::batches::{Batch, BatchCreateParams, BatchListParams};

/// The batches resource.
#[derive(Debug, Clone)]
pub struct Batches {
    client: Client,
}

impl Batches {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create and execute a batch from an uploaded input file.
    pub async fn create(&self, params: BatchCreateParams) -> Result<Batch, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(Method::POST, "/batches", RequestOptions::json(body))
            .await
    }

    /// Retrieve a batch by id.
    pub async fn retrieve(&self, batch_id: &str) -> Result<Batch, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("/batches/{batch_id}"),
                RequestOptions::default(),
            )
            .await
    }

    /// List batches. Fetches a single page.
    pub async fn list(&self, params: Option<BatchListParams>) -> Result<List<Batch>, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .execute(Method::GET, "/batches", RequestOptions::query(query))
            .await
    }

    /// List all batches, following pagination cursors.
    pub async fn list_all(&self) -> Result<Vec<Batch>, OpenAIError> {
        self.client.paginate_all("/batches", Vec::new()).await
    }

    /// Cancel an in-progress batch.
    pub async fn cancel(&self, batch_id: &str) -> Result<Batch, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &format!("/batches/{batch_id}/cancel"),
                RequestOptions::default(),
            )
            .await
    }
}
