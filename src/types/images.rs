//! Image generation types, mirroring
//! `openai-python/src/openai/types/images_response.py` and
//! `image_generate_params.py`.

use serde::{Deserialize, Serialize};

use super::common::null_as_default;

/// Request body for `POST /images/generations`.
#[derive(Debug, Clone, Serialize)]
pub struct ImageGenerationRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    /// e.g. `"1024x1024"`, `"1792x1024"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// e.g. `"standard"`, `"hd"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    /// `"url"` (default) or `"b64_json"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    /// e.g. `"vivid"`, `"natural"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl ImageGenerationRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: None,
            n: None,
            size: None,
            quality: None,
            response_format: None,
            style: None,
            user: None,
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn n(mut self, n: u32) -> Self {
        self.n = Some(n);
        self
    }

    pub fn size(mut self, size: impl Into<String>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn quality(mut self, quality: impl Into<String>) -> Self {
        self.quality = Some(quality.into());
        self
    }

    pub fn response_format(mut self, response_format: impl Into<String>) -> Self {
        self.response_format = Some(response_format.into());
        self
    }

    /// Image style: `"vivid"` or `"natural"` (dall-e-3 only).
    pub fn style(mut self, style: impl Into<String>) -> Self {
        self.style = Some(style.into());
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
}

/// One generated image.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImageData {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub b64_json: Option<String>,
    #[serde(default)]
    pub revised_prompt: Option<String>,
}

/// Response from `POST /images/generations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImagesResponse {
    #[serde(default)]
    pub created: Option<i64>,
    /// Missing or `null` reads as an empty list.
    #[serde(default, deserialize_with = "null_as_default")]
    pub data: Vec<ImageData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_response_data_tolerates_null_and_missing() {
        let null: ImagesResponse = serde_json::from_str(r#"{"created": 1, "data": null}"#).unwrap();
        assert!(null.data.is_empty());
        let missing: ImagesResponse = serde_json::from_str(r#"{"created": 1}"#).unwrap();
        assert!(missing.data.is_empty());
        let present: ImagesResponse =
            serde_json::from_str(r#"{"data": [{"url": "https://x/y.png"}]}"#).unwrap();
        assert_eq!(present.data.len(), 1);
    }
}
