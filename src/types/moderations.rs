//! Moderation types, mirroring `openai-python/src/openai/types/moderation*.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// `input` accepts a string or an array of strings.
/// (Multimodal moderation input is out of scope for v0.1.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModerationInput {
    Text(String),
    Texts(Vec<String>),
}

impl From<&str> for ModerationInput {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for ModerationInput {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<String>> for ModerationInput {
    fn from(texts: Vec<String>) -> Self {
        Self::Texts(texts)
    }
}

/// Request body for `POST /moderations`.
#[derive(Debug, Clone, Serialize)]
pub struct ModerationRequest {
    pub input: ModerationInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl ModerationRequest {
    pub fn new(input: impl Into<ModerationInput>) -> Self {
        Self {
            input: input.into(),
            model: None,
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// One moderation verdict. Category names vary by moderation model, so
/// categories and scores are kept as maps; values may be null for categories
/// a model does not support.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModerationResult {
    pub flagged: bool,
    #[serde(default)]
    pub categories: HashMap<String, Option<bool>>,
    #[serde(default)]
    pub category_scores: HashMap<String, Option<f64>>,
}

/// Response from `POST /moderations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModerationResponse {
    pub id: String,
    pub model: String,
    pub results: Vec<ModerationResult>,
}
