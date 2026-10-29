//! The `Response` object and output items, mirroring
//! `openai-python/src/openai/types/responses/response.py` and
//! `response_output_item.py` (v1 subset: message / function_call / reasoning
//! output items — exotic ones round-trip through `Other`).

use serde::{Deserialize, Serialize};

use crate::pagination::HasId;
use crate::types::common::null_as_default;

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
    /// Forward-compatibility: a status this crate doesn't know yet must not
    /// fail deserialization of the whole [`Response`]. The original string is
    /// not retained, so this serializes as `"unknown"`.
    #[serde(other)]
    Unknown,
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
    #[non_exhaustive]
    OutputText {
        /// The API may send `null` here; it reads as an empty string.
        #[serde(default, deserialize_with = "null_as_default")]
        text: String,
        /// `null` reads as no annotations, so the part stays typed and
        /// `Response::output_text` still sees its text.
        #[serde(default, deserialize_with = "null_as_default")]
        annotations: Vec<serde_json::Value>,
        /// Fields not modelled above (e.g. `logprobs`, `phase`, `caller`),
        /// kept so a parsed item re-serializes without data loss.
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
    #[non_exhaustive]
    Refusal {
        refusal: String,
        /// Fields not modelled above,
        /// kept so a parsed item re-serializes without data loss.
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
}

/// One element of an output message's `content`, mirroring the
/// `OutputItem` pattern: recognized parts are typed, anything else (e.g.
/// `input_text`, `input_image`, `output_audio`) falls into
/// [`OutputContentPart::Other`] with its payload intact, so the enclosing
/// message stays typed and a parsed `Response.output` re-serializes without
/// data loss (object key order is normalized through `serde_json::Value`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputContentPart {
    Known(OutputContent),
    Other(serde_json::Value),
}

/// The typed output item variants this crate models directly. Anything else
/// (computer_call, file_search_call, web_search_call, mcp_call, ...) falls
/// back to [`OutputItem::Other`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum KnownOutputItem {
    #[non_exhaustive]
    Message {
        id: String,
        role: String,
        /// The API may send `null` here; it reads as no parts.
        #[serde(default, deserialize_with = "null_as_default")]
        content: Vec<OutputContentPart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        /// Fields not modelled above (e.g. `logprobs`, `phase`, `caller`),
        /// kept so a parsed item re-serializes without data loss.
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
    #[non_exhaustive]
    FunctionCall {
        /// Absent on some function calls (upstream treats it as optional).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        call_id: String,
        name: String,
        /// JSON-encoded arguments string (may be invalid JSON — validate
        /// before use).
        arguments: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        /// Fields not modelled above (e.g. `logprobs`, `phase`, `caller`),
        /// kept so a parsed item re-serializes without data loss.
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
    #[non_exhaustive]
    Reasoning {
        id: String,
        summary: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<Vec<serde_json::Value>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encrypted_content: Option<String>,
        /// Fields not modelled above (e.g. `logprobs`, `phase`, `caller`),
        /// kept so a parsed item re-serializes without data loss.
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
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
    /// The API may send `output: null`; it reads as an empty list.
    #[serde(default, deserialize_with = "null_as_default")]
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
    /// Fields not modeled above (e.g. `prompt_cache_diagnostics`), kept so
    /// they stay readable and survive re-serialization.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
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
                    if let OutputContentPart::Known(OutputContent::OutputText {
                        text: part_text,
                        ..
                    }) = part
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

/// Result of `POST /responses/compact`: the compacted output items (all user
/// messages followed by one compaction item) and the compaction pass's usage.
/// Not a [`Response`]: there is no `model`, `status` or other response state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CompactedResponse {
    pub id: String,
    pub created_at: i64,
    /// Always `"response.compaction"`.
    #[serde(default)]
    pub object: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub output: Vec<OutputItem>,
    pub usage: ResponseUsage,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Result of `POST /responses/input_tokens`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InputTokenCount {
    pub input_tokens: u64,
    /// Always `"response.input_tokens"`.
    #[serde(default)]
    pub object: String,
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

    #[test]
    fn untyped_fields_land_in_extra_without_duplicating_typed_keys() {
        let mut body = sample_response_json();
        body["prompt_cache_diagnostics"] = serde_json::json!({"reused_tokens": 128});
        let response: Response = serde_json::from_value(body).unwrap();
        assert_eq!(
            response.extra["prompt_cache_diagnostics"]["reused_tokens"],
            128
        );
        assert!(!response.extra.contains_key("id"));
        assert!(!response.extra.contains_key("model"));

        let text = serde_json::to_string(&response).unwrap();
        assert_eq!(text.matches("\"id\":\"resp_123\"").count(), 1);
        assert_eq!(text.matches("\"prompt_cache_diagnostics\"").count(), 1);
        let back: Response = serde_json::from_str(&text).unwrap();
        assert_eq!(back.extra, response.extra);
    }

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

    #[test]
    fn null_output_parses_as_empty() {
        let json = serde_json::json!({
            "id": "resp_1", "created_at": 1.0, "model": "gpt-5", "output": null
        });
        let response: Response = serde_json::from_value(json).unwrap();
        assert!(response.output.is_empty());
        assert_eq!(response.output_text(), "");
    }

    #[test]
    fn null_content_and_null_text_keep_messages_typed() {
        let json = serde_json::json!({
            "id": "resp_1", "created_at": 1.0, "model": "gpt-5",
            "output": [
                {"type": "message", "id": "msg_1", "role": "assistant", "content": null},
                {"type": "message", "id": "msg_2", "role": "assistant", "content": [
                    {"type": "output_text", "text": null, "annotations": []},
                    {"type": "output_text", "text": "kept", "annotations": []}
                ]}
            ]
        });
        let response: Response = serde_json::from_value(json).unwrap();
        assert_eq!(response.output.len(), 2);
        match &response.output[0] {
            OutputItem::Known(KnownOutputItem::Message { content, .. }) => {
                assert!(content.is_empty())
            }
            other => panic!("expected typed message, got {other:?}"),
        }
        match &response.output[1] {
            OutputItem::Known(KnownOutputItem::Message { content, .. }) => {
                assert_eq!(content.len(), 2);
                assert!(matches!(
                    &content[0],
                    OutputContentPart::Known(OutputContent::OutputText { text, .. })
                        if text.is_empty()
                ));
            }
            other => panic!("expected typed message, got {other:?}"),
        }
        assert_eq!(response.output_text(), "kept");
    }

    #[test]
    fn unknown_status_does_not_fail_response() {
        let json = serde_json::json!({
            "id": "resp_1", "created_at": 1.0, "model": "gpt-5", "status": "brand_new_state"
        });
        let response: Response = serde_json::from_value(json).unwrap();
        assert_eq!(response.status, Some(ResponseStatus::Unknown));
    }

    #[test]
    fn unknown_content_part_keeps_message_typed() {
        let json = serde_json::json!({
            "id": "resp_1", "created_at": 1.0, "model": "gpt-5",
            "output": [{"type": "message", "id": "msg_1", "role": "assistant", "content": [
                {"type": "audio_thing", "data": "xyz"},
                {"type": "output_text", "text": "still here", "annotations": []}
            ]}]
        });
        let response: Response = serde_json::from_value(json).unwrap();
        match &response.output[0] {
            OutputItem::Known(KnownOutputItem::Message { content, .. }) => {
                assert!(matches!(content[0], OutputContentPart::Other(_)));
            }
            other => panic!("expected typed message, got {other:?}"),
        }
        assert_eq!(response.output_text(), "still here");
    }

    #[test]
    fn unknown_content_part_round_trips_exactly() {
        let output = serde_json::json!([{
            "type": "message", "id": "msg_1", "role": "assistant", "status": "completed",
            "content": [
                {"type": "output_audio", "data": "AAEC", "transcript": "hi"},
                {"type": "output_text", "text": "x", "annotations": []}
            ]
        }]);
        let json = serde_json::json!({
            "id": "resp_1", "created_at": 1.0, "model": "gpt-5", "output": output
        });
        let response: Response = serde_json::from_value(json).unwrap();
        assert!(matches!(
            response.output[0],
            OutputItem::Known(KnownOutputItem::Message { .. })
        ));
        assert_eq!(serde_json::to_value(&response.output).unwrap(), output);
    }

    #[test]
    fn known_items_keep_unmodelled_fields_on_round_trip() {
        let output = serde_json::json!([
            {
                "type": "message", "id": "msg_1", "role": "assistant", "status": "completed",
                "phase": "final_answer",
                "content": [{
                    "type": "output_text", "text": "x", "annotations": [],
                    "logprobs": [{"token": "x", "logprob": -0.1, "bytes": [120], "top_logprobs": []}]
                }]
            },
            {
                "type": "function_call", "id": "fc_1", "call_id": "call_1", "name": "f",
                "arguments": "{}", "namespace": "ns", "async": true, "caller": {"type": "direct"}
            },
            {"type": "reasoning", "id": "rs_1", "summary": [], "status": "completed"}
        ]);
        let json = serde_json::json!({
            "id": "resp_1", "created_at": 1.0, "model": "gpt-5", "output": output
        });
        let response: Response = serde_json::from_value(json).unwrap();
        assert!(response
            .output
            .iter()
            .all(|item| matches!(item, OutputItem::Known(_))));
        match &response.output[0] {
            OutputItem::Known(KnownOutputItem::Message { content, extra, .. }) => {
                assert_eq!(extra["phase"], "final_answer");
                assert!(matches!(
                    &content[0],
                    OutputContentPart::Known(OutputContent::OutputText { extra, .. })
                        if extra.contains_key("logprobs")
                ));
            }
            other => panic!("expected typed message, got {other:?}"),
        }
        assert_eq!(serde_json::to_value(&response.output).unwrap(), output);
        // Serialize to text, where a leaked tag would show as a duplicate key.
        let text = serde_json::to_string(&response.output).unwrap();
        assert_eq!(text.matches(r#""type":"message""#).count(), 1);
        assert_eq!(text.matches(r#""type":"function_call""#).count(), 1);
        assert_eq!(text.matches(r#""type":"reasoning""#).count(), 1);
    }

    #[test]
    fn output_text_with_null_annotations_stays_typed() {
        let json = serde_json::json!({
            "id": "resp_1", "created_at": 1.0, "model": "gpt-5",
            "output": [{
                "type": "message", "id": "msg_1", "role": "assistant",
                "content": [{"type": "output_text", "text": "still here", "annotations": null}]
            }]
        });
        let response: Response = serde_json::from_value(json).unwrap();
        assert_eq!(response.output_text(), "still here");
    }

    #[test]
    fn function_call_without_id_is_returned() {
        let json = serde_json::json!({
            "id": "resp_1", "created_at": 1.0, "model": "gpt-5",
            "output": [{
                "type": "function_call", "call_id": "call_1",
                "name": "get_weather", "arguments": "{}"
            }]
        });
        let response: Response = serde_json::from_value(json).unwrap();
        let calls = response.function_calls();
        assert_eq!(calls.len(), 1);
        assert!(matches!(
            calls[0],
            KnownOutputItem::FunctionCall { id: None, .. }
        ));
    }
}
