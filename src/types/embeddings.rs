//! Embeddings types, mirroring `openai-python/src/openai/types/embedding*.py`.

use serde::{Deserialize, Serialize};

use super::common::Usage;

/// `input` accepts a string, an array of strings, or (arrays of) token ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Text(String),
    Texts(Vec<String>),
    Tokens(Vec<i64>),
    TokenArrays(Vec<Vec<i64>>),
}

impl From<&str> for EmbeddingInput {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for EmbeddingInput {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<String>> for EmbeddingInput {
    fn from(texts: Vec<String>) -> Self {
        Self::Texts(texts)
    }
}

/// Request body for `POST /embeddings` (`resources/embeddings.py::create`).
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: EmbeddingInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    /// `"float"` (default) or `"base64"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl EmbeddingRequest {
    pub fn new(model: impl Into<String>, input: impl Into<EmbeddingInput>) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            dimensions: None,
            encoding_format: None,
            user: None,
        }
    }

    pub fn dimensions(mut self, dimensions: u32) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    pub fn encoding_format(mut self, encoding_format: impl Into<String>) -> Self {
        self.encoding_format = Some(encoding_format.into());
        self
    }
}

/// The embedding vector: floats by default, or a base64 string when
/// `encoding_format: "base64"` was requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingVector {
    Floats(Vec<f32>),
    Base64(String),
}

/// One embedding result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Embedding {
    pub index: u32,
    pub embedding: EmbeddingVector,
    #[serde(default)]
    pub object: String,
}

/// Response from `POST /embeddings`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbeddingResponse {
    pub data: Vec<Embedding>,
    pub model: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_variants_serialize() {
        assert_eq!(
            serde_json::to_value(EmbeddingInput::from("hi")).unwrap(),
            serde_json::json!("hi")
        );
        assert_eq!(
            serde_json::to_value(EmbeddingInput::Texts(vec!["a".into(), "b".into()])).unwrap(),
            serde_json::json!(["a", "b"])
        );
        assert_eq!(
            serde_json::to_value(EmbeddingInput::Tokens(vec![1, 2])).unwrap(),
            serde_json::json!([1, 2])
        );
    }

    #[test]
    fn response_deserializes_floats() {
        let body = r#"{
            "object": "list",
            "data": [{"object": "embedding", "index": 0, "embedding": [0.1, -0.2]}],
            "model": "text-embedding-3-small",
            "usage": {"prompt_tokens": 5, "total_tokens": 5}
        }"#;
        let response: EmbeddingResponse = serde_json::from_str(body).unwrap();
        let EmbeddingVector::Floats(floats) = &response.data[0].embedding else {
            panic!("expected float vector");
        };
        assert_eq!(floats.len(), 2);
    }
}
