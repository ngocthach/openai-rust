//! Cursor pagination, mirroring `pagination.py::SyncCursorPage`: pages carry
//! `data` + `has_more`, and the next page is requested with
//! `after = <last item id>`.

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

impl Client {
    /// Fetch every item from a cursor-paginated list endpoint, following
    /// `after` cursors until `has_more` is false or a page is empty.
    pub(crate) async fn paginate_all<T: HasId + DeserializeOwned>(
        &self,
        path: &str,
        mut query: Vec<(String, String)>,
    ) -> Result<Vec<T>, OpenAIError> {
        let mut items: Vec<T> = Vec::new();
        let mut previous_cursor: Option<String> = None;
        loop {
            let page: List<T> = self
                .execute(
                    reqwest::Method::GET,
                    path,
                    RequestOptions::query(query.clone()),
                )
                .await?;
            let has_next = page.has_next_page();
            items.extend(page.data);

            if !has_next {
                return Ok(items);
            }
            let Some(cursor) = items.last().and_then(HasId::id).map(str::to_owned) else {
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
