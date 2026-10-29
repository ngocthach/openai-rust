//! Cursor pagination, mirroring `pagination.py::SyncCursorPage`: pages carry
//! `data` + `has_more`, and the next page is requested with
//! `after = <last item id>`. Conversation-style pages
//! (`AsyncConversationCursorPage`) instead advance with the envelope's
//! `last_id`.

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::RequestOptions;

/// A single page of results from a list endpoint.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct List<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub has_more: Option<bool>,
    #[serde(default)]
    pub first_id: Option<String>,
    #[serde(default)]
    pub last_id: Option<String>,
}

impl<T> List<T> {
    /// Whether another page may exist, mirroring
    /// `SyncCursorPage::has_next_page`.
    pub fn has_next_page(&self) -> bool {
        !self.data.is_empty() && self.has_more != Some(false)
    }
}

/// Items that expose an `id` usable as an `after` cursor.
pub trait HasId {
    fn id(&self) -> Option<&str>;
}

/// Build the cursor query pairs (`after`/`before`/`limit`/`order`) shared by
/// list endpoints; module-specific params are appended by the caller.
pub(crate) fn cursor_query(
    after: Option<&str>,
    before: Option<&str>,
    limit: Option<u32>,
    order: Option<&str>,
) -> Vec<(String, String)> {
    let mut query = Vec::new();
    if let Some(after) = after {
        query.push(("after".to_string(), after.to_string()));
    }
    if let Some(before) = before {
        query.push(("before".to_string(), before.to_string()));
    }
    if let Some(limit) = limit {
        query.push(("limit".to_string(), limit.to_string()));
    }
    if let Some(order) = order {
        query.push(("order".to_string(), order.to_string()));
    }
    query
}

impl Client {
    /// Fetch every item from a cursor-paginated list endpoint, following
    /// `after` cursors until `has_more` is false or a page is empty.
    pub(crate) async fn paginate_all<T: HasId + DeserializeOwned>(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<Vec<T>, OpenAIError> {
        self.paginate_all_with_headers(path, query, Vec::new())
            .await
    }

    /// [`Client::paginate_all`] with extra headers sent on every page request
    /// (used by beta endpoints that require `OpenAI-Beta: assistants=v2`).
    pub(crate) async fn paginate_all_with_headers<T: HasId + DeserializeOwned>(
        &self,
        path: &str,
        query: Vec<(String, String)>,
        extra_headers: Vec<(String, String)>,
    ) -> Result<Vec<T>, OpenAIError> {
        self.paginate_pages(
            path,
            query,
            extra_headers,
            Some(|item: &T| item.id().map(str::to_owned)),
        )
        .await
    }

    /// Like [`Client::paginate_all`] for endpoints whose pages advance with
    /// the envelope `last_id` (conversation items), which may differ from the
    /// id of the last item in `data`. A page that reports `has_more: true`
    /// without a `last_id` is an error rather than a silent truncation.
    pub(crate) async fn paginate_all_by_last_id<T: DeserializeOwned>(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<Vec<T>, OpenAIError> {
        self.paginate_pages::<T>(path, query, Vec::new(), None)
            .await
    }

    async fn paginate_pages<T: DeserializeOwned>(
        &self,
        path: &str,
        mut query: Vec<(String, String)>,
        extra_headers: Vec<(String, String)>,
        // `None` advances with the envelope `last_id` instead of an item id.
        item_cursor: Option<fn(&T) -> Option<String>>,
    ) -> Result<Vec<T>, OpenAIError> {
        let mut items: Vec<T> = Vec::new();
        let mut previous_cursor: Option<String> = None;
        loop {
            let options = RequestOptions {
                query: query.clone(),
                extra_headers: extra_headers.clone(),
                ..RequestOptions::default()
            };
            let page: List<T> = self.execute(reqwest::Method::GET, path, options).await?;
            let has_next = page.has_next_page();
            let envelope_cursor = page.last_id.clone();
            let has_more = page.has_more;
            items.extend(page.data);

            if !has_next {
                return Ok(items);
            }
            let cursor = match item_cursor {
                Some(item_cursor) => items.last().and_then(item_cursor),
                None => envelope_cursor,
            };
            let Some(cursor) = cursor else {
                if item_cursor.is_none() && has_more == Some(true) {
                    return Err(OpenAIError::Config(
                        "page reported has_more but carried no last_id to continue from".into(),
                    ));
                }
                return Ok(items);
            };
            // Guard against providers that ignore `after` (or omit
            // `has_more`): a cursor that does not advance would loop forever.
            if previous_cursor.as_deref() == Some(cursor.as_str()) {
                return Ok(items);
            }
            previous_cursor = Some(cursor.clone());
            query.retain(|(k, _)| k != "after");
            query.push(("after".into(), cursor));
        }
    }
}
