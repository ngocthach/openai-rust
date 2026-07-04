//! `/vector_stores` resource, mirroring
//! `openai-python/src/openai/resources/vector_stores/`. Covers the store CRUD +
//! search endpoints plus the `.files(id)` and `.file_batches(id)` sub-resources.
//!
//! Every vector-store endpoint requires the beta header
//! `OpenAI-Beta: assistants=v2`; it is attached to every request here.

use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::pagination::{HasId, List};
use crate::request::RequestOptions;
use crate::types::vector_stores::{
    FileContentResponse, VectorStore, VectorStoreCreateParams, VectorStoreDeleted, VectorStoreFile,
    VectorStoreFileBatch, VectorStoreFileBatchCreateParams, VectorStoreFileCreateParams,
    VectorStoreFileDeleted, VectorStoreFileListParams, VectorStoreListParams,
    VectorStoreSearchParams, VectorStoreSearchResponse, VectorStoreUpdateParams,
};

/// Attach the required beta header to a set of request options.
fn with_beta(mut options: RequestOptions) -> RequestOptions {
    options
        .extra_headers
        .push(("OpenAI-Beta".to_string(), "assistants=v2".to_string()));
    options
}

/// Build JSON request options from a serializable body, plus the beta header.
fn beta_json<B: serde::Serialize>(body: &B) -> Result<RequestOptions, OpenAIError> {
    Ok(with_beta(RequestOptions::json(serde_json::to_value(body)?)))
}

/// Paginate a cursor list endpoint while sending the beta header on every page.
async fn paginate_all_beta<T: HasId + DeserializeOwned>(
    client: &Client,
    path: &str,
    mut query: Vec<(String, String)>,
) -> Result<Vec<T>, OpenAIError> {
    let mut items: Vec<T> = Vec::new();
    let mut previous_cursor: Option<String> = None;
    loop {
        let page: List<T> = client
            .execute(Method::GET, path, with_beta(RequestOptions::query(query.clone())))
            .await?;
        let has_next = page.has_next_page();
        items.extend(page.data);
        if !has_next {
            return Ok(items);
        }
        let Some(cursor) = items.last().and_then(HasId::id).map(str::to_owned) else {
            return Ok(items);
        };
        if previous_cursor.as_deref() == Some(cursor.as_str()) {
            return Ok(items);
        }
        previous_cursor = Some(cursor.clone());
        query.retain(|(k, _)| k != "after");
        query.push(("after".into(), cursor));
    }
}

/// The vector stores resource.
#[derive(Debug, Clone)]
pub struct VectorStores {
    client: Client,
}

impl VectorStores {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a vector store.
    pub async fn create(
        &self,
        params: VectorStoreCreateParams,
    ) -> Result<VectorStore, OpenAIError> {
        self.client
            .execute(Method::POST, "/vector_stores", beta_json(&params)?)
            .await
    }

    /// Retrieve a vector store by id.
    pub async fn retrieve(&self, vector_store_id: &str) -> Result<VectorStore, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("/vector_stores/{vector_store_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Update a vector store (POST, not PATCH).
    pub async fn update(
        &self,
        vector_store_id: &str,
        params: VectorStoreUpdateParams,
    ) -> Result<VectorStore, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &format!("/vector_stores/{vector_store_id}"),
                beta_json(&params)?,
            )
            .await
    }

    /// List vector stores (single page).
    pub async fn list(
        &self,
        params: Option<VectorStoreListParams>,
    ) -> Result<List<VectorStore>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                "/vector_stores",
                with_beta(RequestOptions::query(query)),
            )
            .await
    }

    /// List all vector stores, following pagination cursors.
    pub async fn list_all(&self) -> Result<Vec<VectorStore>, OpenAIError> {
        paginate_all_beta(&self.client, "/vector_stores", Vec::new()).await
    }

    /// Delete a vector store.
    pub async fn delete(
        &self,
        vector_store_id: &str,
    ) -> Result<VectorStoreDeleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &format!("/vector_stores/{vector_store_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Search a vector store for relevant chunks (POST, returns a single page).
    pub async fn search(
        &self,
        vector_store_id: &str,
        params: VectorStoreSearchParams,
    ) -> Result<List<VectorStoreSearchResponse>, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &format!("/vector_stores/{vector_store_id}/search"),
                beta_json(&params)?,
            )
            .await
    }

    /// Files sub-resource bound to a vector store id.
    pub fn files(&self, vector_store_id: impl Into<String>) -> VectorStoreFiles {
        VectorStoreFiles::new(self.client.clone(), vector_store_id.into())
    }

    /// File batches sub-resource bound to a vector store id.
    pub fn file_batches(
        &self,
        vector_store_id: impl Into<String>,
    ) -> VectorStoreFileBatches {
        VectorStoreFileBatches::new(self.client.clone(), vector_store_id.into())
    }
}

/// The files sub-resource of a single vector store.
#[derive(Debug, Clone)]
pub struct VectorStoreFiles {
    client: Client,
    vector_store_id: String,
}

impl VectorStoreFiles {
    pub(crate) fn new(client: Client, vector_store_id: String) -> Self {
        Self {
            client,
            vector_store_id,
        }
    }

    fn base(&self) -> String {
        format!("/vector_stores/{}/files", self.vector_store_id)
    }

    /// Attach a file to the vector store.
    pub async fn create(
        &self,
        params: VectorStoreFileCreateParams,
    ) -> Result<VectorStoreFile, OpenAIError> {
        self.client
            .execute(Method::POST, &self.base(), beta_json(&params)?)
            .await
    }

    /// Retrieve a vector store file.
    pub async fn retrieve(&self, file_id: &str) -> Result<VectorStoreFile, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("{}/{file_id}", self.base()),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Update a vector store file's attributes (POST).
    pub async fn update(
        &self,
        file_id: &str,
        attributes: serde_json::Value,
    ) -> Result<VectorStoreFile, OpenAIError> {
        let body = serde_json::json!({ "attributes": attributes });
        self.client
            .execute(
                Method::POST,
                &format!("{}/{file_id}", self.base()),
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// Remove a file from the vector store.
    pub async fn delete(
        &self,
        file_id: &str,
    ) -> Result<VectorStoreFileDeleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &format!("{}/{file_id}", self.base()),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// List files in the vector store (single page).
    pub async fn list(
        &self,
        params: Option<VectorStoreFileListParams>,
    ) -> Result<List<VectorStoreFile>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                &self.base(),
                with_beta(RequestOptions::query(query)),
            )
            .await
    }

    /// List all files in the vector store, following pagination cursors.
    pub async fn list_all(
        &self,
        params: Option<VectorStoreFileListParams>,
    ) -> Result<Vec<VectorStoreFile>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        paginate_all_beta(&self.client, &self.base(), query).await
    }

    /// Retrieve the parsed content of a vector store file.
    pub async fn content(
        &self,
        file_id: &str,
    ) -> Result<List<FileContentResponse>, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("{}/{file_id}/content", self.base()),
                with_beta(RequestOptions::default()),
            )
            .await
    }
}

/// The file batches sub-resource of a single vector store.
#[derive(Debug, Clone)]
pub struct VectorStoreFileBatches {
    client: Client,
    vector_store_id: String,
}

impl VectorStoreFileBatches {
    pub(crate) fn new(client: Client, vector_store_id: String) -> Self {
        Self {
            client,
            vector_store_id,
        }
    }

    fn base(&self) -> String {
        format!("/vector_stores/{}/file_batches", self.vector_store_id)
    }

    /// Create a file batch.
    pub async fn create(
        &self,
        params: VectorStoreFileBatchCreateParams,
    ) -> Result<VectorStoreFileBatch, OpenAIError> {
        self.client
            .execute(Method::POST, &self.base(), beta_json(&params)?)
            .await
    }

    /// Retrieve a file batch.
    pub async fn retrieve(
        &self,
        batch_id: &str,
    ) -> Result<VectorStoreFileBatch, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("{}/{batch_id}", self.base()),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Cancel a file batch (POST, no body).
    pub async fn cancel(
        &self,
        batch_id: &str,
    ) -> Result<VectorStoreFileBatch, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &format!("{}/{batch_id}/cancel", self.base()),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// List the files in a file batch (single page).
    pub async fn list_files(
        &self,
        batch_id: &str,
        params: Option<VectorStoreFileListParams>,
    ) -> Result<List<VectorStoreFile>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                &format!("{}/{batch_id}/files", self.base()),
                with_beta(RequestOptions::query(query)),
            )
            .await
    }
}
