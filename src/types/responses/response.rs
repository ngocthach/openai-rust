//! The `Response` object and output items, mirroring
//! `openai-python/src/openai/types/responses/response.py` and
//! `response_output_item.py` (v1 subset: message / function_call / reasoning
//! output items — exotic ones round-trip through `Other`).

use serde::{Deserialize, Serialize};

use crate::pagination::HasId;

/// Lifecycle status of a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ResponseStatus {
    Completed,
    Failed,
    InProgress,
    Cancelled,
    Queued,
    Incomplete,
}

/// Error returned when the model fails to generate a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ResponseError {
    pub code: String,
    pub message: String,
}

/// Why an otherwise-successful response is incomplete.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IncompleteDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Token usage for a response, mirroring `ResponseUsage`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ResponseUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens_details: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<serde_json::Value>,
}

/// One part of an output message's content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum OutputContent {
    OutputText {
        text: String,
        #[serde(default)]
        annotations: Vec<serde_json::Value>,
    },
    Refusal {
        refusal: String,
    },
}

/// The typed output item variants this crate models directly. Anything else
/// (computer_call, file_search_call, web_search_call, mcp_call, ...) falls
/// back to [`OutputItem::Other`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum KnownOutputItem {
    Message {
        id: String,
        role: String,
        content: Vec<OutputContent>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
    FunctionCall {
        id: String,
        call_id: String,
        name: String,
        /// JSON-encoded arguments string (may be invalid JSON — validate
        /// before use).
        arguments: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
    Reasoning {
        id: String,
        summary: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<Vec<serde_json::Value>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encrypted_content: Option<String>,
    },
}

/// One item in `Response.output`, mirroring `ResponseOutputItem`.
///
/// Unrecognized `type` values deserialize into [`OutputItem::Other`] and
/// round-trip without data loss (object key order is normalized through
/// `serde_json::Value`, but no field is dropped).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputItem {
    Known(KnownOutputItem),
    Other(serde_json::Value),
}

/// Response from `POST /responses`, `GET /responses/{id}`, and the terminal
/// stream events (`Completed`/`Failed`/`Incomplete`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Response {
    pub id: String,
    pub created_at: f64,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub status: Option<ResponseStatus>,
    pub model: String,
    #[serde(default)]
    pub output: Vec<OutputItem>,
    #[serde(default)]
    pub error: Option<ResponseError>,
    #[serde(default)]
    pub incomplete_details: Option<IncompleteDetails>,
    #[serde(default)]
    pub usage: Option<ResponseUsage>,
    #[serde(default)]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub previous_response_id: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
}

impl Response {
    /// Concatenated text of every `output_text` content part across all
    /// output messages — the common case for reading a response's answer.
    /// Mirrors the Python SDK's `Response.output_text` computed property.
    pub fn output_text(&self) -> String {
        let mut text = String::new();
        for item in &self.output {
            if let OutputItem::Known(KnownOutputItem::Message { content, .. }) = item {
                for part in content {
                    if let OutputContent::OutputText {
                        text: part_text, ..
                    } = part
                    {
                        text.push_str(part_text);
                    }
                }
            }
        }
        text
    }

    /// Function calls the model made in this response, if any.
    pub fn function_calls(&self) -> Vec<&KnownOutputItem> {
        self.output
            .iter()
            .filter_map(|item| match item {
                OutputItem::Known(known @ KnownOutputItem::FunctionCall { .. }) => Some(known),
                _ => None,
            })
            .collect()
    }
}

/// One item returned by `GET /responses/{id}/input_items`. Kept as raw JSON
/// since input items may be any of the input or output item shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResponseItem(pub serde_json::Value);

impl HasId for ResponseItem {
    fn id(&self) -> Option<&str> {
        self.0.get("id").and_then(serde_json::Value::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_response_json() -> serde_json::Value {
        serde_json::json!({
            "id": "resp_123",
            "object": "response",
            "created_at": 1728933352.0,
            "status": "completed",
            "model": "gpt-5",
            "output": [{
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Hello there!", "annotations": []}]
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "total_tokens": 15,
                "input_tokens_details": {"cached_tokens": 0},
                "output_tokens_details": {"reasoning_tokens": 0}
            }
        })
    }

    #[test]
    fn deserializes_real_response() {
        let response: Response = serde_json::from_value(sample_response_json()).unwrap();
        assert_eq!(response.status, Some(ResponseStatus::Completed));
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, 15);
    }

    #[test]
    fn output_text_concatenates() {
        let response: Response = serde_json::from_value(sample_response_json()).unwrap();
        assert_eq!(response.output_text(), "Hello there!");
    }

    #[test]
    fn function_call_output_item_parses() {
        let json = serde_json::json!({
            "id": "resp_1", "object": "response", "created_at": 1.0, "model": "gpt-5",
            "output": [{
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_1",
                "name": "get_weather",
                "arguments": "{\"city\":\"Hanoi\"}"
            }]
        });
        let response: Response = serde_json::from_value(json).unwrap();
        let calls = response.function_calls();
        assert_eq!(calls.len(), 1);
        match calls[0] {
            KnownOutputItem::FunctionCall { name, .. } => assert_eq!(name, "get_weather"),
            _ => panic!("expected FunctionCall"),
        }
    }

    #[test]
    fn unknown_output_item_preserved_as_other() {
        let json = serde_json::json!({
            "type": "web_search_call",
            "id": "ws_1",
            "status": "completed"
        });
        let item: OutputItem = serde_json::from_value(json.clone()).unwrap();
        assert!(matches!(item, OutputItem::Other(_)));
        assert_eq!(serde_json::to_value(&item).unwrap(), json);
    }
}
