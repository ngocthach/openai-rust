//! `/responses` resource, mirroring `resources/responses/responses.py`:
//! create (+streaming), retrieve (+streaming resume for background mode),
//! delete, cancel, and the `input_items` sub-resource.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::pagination::List;
use crate::request::RequestOptions;
use crate::streaming::EventStream;
use crate::types::responses::{
    CreateResponseRequest, Response, ResponseIncludable, ResponseItem, ResponseStreamEvent,
};

/// Query parameters for `retrieve`.
#[derive(Debug, Clone, Default)]
pub struct RetrieveParams {
    pub include: Option<Vec<ResponseIncludable>>,
    pub include_obfuscation: Option<bool>,
}

impl RetrieveParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn include(mut self, include: Vec<ResponseIncludable>) -> Self {
        self.include = Some(include);
        self
    }

    pub fn include_obfuscation(mut self, include_obfuscation: bool) -> Self {
        self.include_obfuscation = Some(include_obfuscation);
        self
    }

    fn to_query(&self) -> Vec<(String, String)> {
        let mut query = Vec::new();
        if let Some(include) = &self.include {
            for item in include {
                query.push(("include[]".into(), item.as_str().to_string()));
            }
        }
        if let Some(include_obfuscation) = self.include_obfuscation {
            query.push((
                "include_obfuscation".into(),
                include_obfuscation.to_string(),
            ));
        }
        query
    }
}

/// Query parameters for `input_items().list()`.
#[derive(Debug, Clone, Default)]
pub struct InputItemsListParams {
    pub after: Option<String>,
    pub before: Option<String>,
    pub limit: Option<u32>,
    pub order: Option<String>,
    pub include: Option<Vec<ResponseIncludable>>,
}

impl InputItemsListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sort order: `"asc"` or `"desc"`.
    pub fn order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    pub fn include(mut self, include: Vec<ResponseIncludable>) -> Self {
        self.include = Some(include);
        self
    }

    fn to_query(&self) -> Vec<(String, String)> {
        let mut query = crate::pagination::cursor_query(
            self.after.as_deref(),
            self.before.as_deref(),
            self.limit,
            self.order.as_deref(),
        );
        if let Some(include) = &self.include {
            for item in include {
                query.push(("include[]".into(), item.as_str().to_string()));
            }
        }
        query
    }
}

/// The `/responses` resource.
#[derive(Debug, Clone)]
pub struct Responses {
    client: Client,
}

impl Responses {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a response and wait for the full result.
    pub async fn create(
        &self,
        mut request: CreateResponseRequest,
    ) -> Result<Response, OpenAIError> {
        request.stream = None;
        let body = serde_json::to_value(&request)?;
        self.client
            .execute(Method::POST, "/responses", RequestOptions::json(body))
            .await
    }

    /// Create a response and stream events as they are generated.
    pub async fn create_stream(
        &self,
        mut request: CreateResponseRequest,
    ) -> Result<EventStream<ResponseStreamEvent>, OpenAIError> {
        request.stream = Some(true);
        let body = serde_json::to_value(&request)?;
        let response = self
            .client
            .execute_raw(Method::POST, "/responses", RequestOptions::json(body))
            .await?;
        Ok(EventStream::new(response))
    }

    /// Retrieve a response by id.
    pub async fn retrieve(
        &self,
        response_id: &str,
        params: Option<RetrieveParams>,
    ) -> Result<Response, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .execute(
                Method::GET,
                &format!("/responses/{response_id}"),
                RequestOptions::query(query),
            )
            .await
    }

    /// Resume streaming a background response, optionally starting after a
    /// given sequence number.
    pub async fn retrieve_stream(
        &self,
        response_id: &str,
        starting_after: Option<u64>,
    ) -> Result<EventStream<ResponseStreamEvent>, OpenAIError> {
        let mut query = vec![("stream".to_string(), "true".to_string())];
        if let Some(starting_after) = starting_after {
            query.push(("starting_after".to_string(), starting_after.to_string()));
        }
        let response = self
            .client
            .execute_raw(
                Method::GET,
                &format!("/responses/{response_id}"),
                RequestOptions::query(query),
            )
            .await?;
        Ok(EventStream::new(response))
    }

    /// Delete a response.
    pub async fn delete(&self, response_id: &str) -> Result<(), OpenAIError> {
        self.client
            .execute_no_content(
                Method::DELETE,
                &format!("/responses/{response_id}"),
                RequestOptions::default(),
            )
            .await
    }

    /// Cancel an in-progress background response.
    pub async fn cancel(&self, response_id: &str) -> Result<Response, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &format!("/responses/{response_id}/cancel"),
                RequestOptions::default(),
            )
            .await
    }

    /// The `input_items` sub-resource for `response_id`.
    pub fn input_items(&self, response_id: impl Into<String>) -> ResponseInputItems {
        ResponseInputItems {
            client: self.client.clone(),
            response_id: response_id.into(),
        }
    }
}

/// `GET /responses/{response_id}/input_items` — cursor-paginated list of the
/// items that produced a response.
#[derive(Debug, Clone)]
pub struct ResponseInputItems {
    client: Client,
    response_id: String,
}

impl ResponseInputItems {
    /// Fetch a single page.
    pub async fn list(
        &self,
        params: Option<InputItemsListParams>,
    ) -> Result<List<ResponseItem>, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .execute(
                Method::GET,
                &format!("/responses/{}/input_items", self.response_id),
                RequestOptions::query(query),
            )
            .await
    }

    /// Fetch every item, following pagination cursors.
    pub async fn list_all(
        &self,
        params: Option<InputItemsListParams>,
    ) -> Result<Vec<ResponseItem>, OpenAIError> {
        let query = params.map(|p| p.to_query()).unwrap_or_default();
        self.client
            .paginate_all(
                &format!("/responses/{}/input_items", self.response_id),
                query,
            )
            .await
    }
}
