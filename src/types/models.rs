//! Model types, mirroring `openai-python/src/openai/types/model.py`.

use serde::{Deserialize, Serialize};

use crate::pagination::HasId;

/// A model available through the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Model {
    pub id: String,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub owned_by: Option<String>,
}

impl HasId for Model {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// Response from `DELETE` endpoints (`{ id, deleted, object }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Deleted {
    pub id: String,
    pub deleted: bool,
    #[serde(default)]
    pub object: String,
}
