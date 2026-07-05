//! Streaming events for `POST /responses` with `stream: true`, mirroring
//! `openai-python/src/openai/types/responses/response_stream_event.py`.
//!
//! Unlike chat completions streaming (homogeneous delta chunks), the
//! Responses API streams ~50 distinct, differently-shaped event types. This
//! crate types the ~12 events needed to drive a typical create/consume loop;
//! everything else deserializes into [`ResponseStreamEvent::Unknown`] rather
//! than failing the stream, so newly-added upstream event types are forward
//! compatible.
//!
//! `response.failed`/`response.incomplete` are typed `Ok` variants, not a
//! stream [`crate::OpenAIError`] — the error detail is nested under
//! `response.error`, so callers must inspect the variant themselves.

use serde::Deserialize;

use super::response::{OutputItem, Response};

/// One event in a Responses API SSE stream.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ResponseStreamEvent {
    #[serde(rename = "response.created")]
    Created {
        response: Response,
        sequence_number: u64,
    },
    #[serde(rename = "response.in_progress")]
    InProgress {
        response: Response,
        sequence_number: u64,
    },
    #[serde(rename = "response.queued")]
    Queued {
        response: Response,
        sequence_number: u64,
    },
    /// Terminal: the response finished successfully.
    #[serde(rename = "response.completed")]
    Completed {
        response: Response,
        sequence_number: u64,
    },
    /// Terminal: the response failed. This is `Ok(Failed{..})`, not a stream
    /// error — inspect `response.error` for details.
    #[serde(rename = "response.failed")]
    Failed {
        response: Response,
        sequence_number: u64,
    },
    /// Terminal: the response stopped early (e.g. hit `max_output_tokens`).
    #[serde(rename = "response.incomplete")]
    Incomplete {
        response: Response,
        sequence_number: u64,
    },
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded {
        output_index: u32,
        item: OutputItem,
        sequence_number: u64,
    },
    #[serde(rename = "response.output_item.done")]
    OutputItemDone {
        output_index: u32,
        item: OutputItem,
        sequence_number: u64,
    },
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded {
        item_id: String,
        output_index: u32,
        content_index: u32,
        part: serde_json::Value,
        sequence_number: u64,
    },
    #[serde(rename = "response.content_part.done")]
    ContentPartDone {
        item_id: String,
        output_index: u32,
        content_index: u32,
        part: serde_json::Value,
        sequence_number: u64,
    },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta {
        item_id: String,
        output_index: u32,
        content_index: u32,
        delta: String,
        sequence_number: u64,
    },
    #[serde(rename = "response.output_text.done")]
    OutputTextDone {
        item_id: String,
        output_index: u32,
        content_index: u32,
        text: String,
        sequence_number: u64,
    },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta {
        item_id: String,
        output_index: u32,
        delta: String,
        sequence_number: u64,
    },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        item_id: String,
        output_index: u32,
        arguments: String,
        sequence_number: u64,
    },
    #[serde(rename = "error")]
    Error {
        #[serde(default)]
        code: Option<String>,
        message: String,
        #[serde(default)]
        param: Option<String>,
        sequence_number: u64,
    },
    /// Any event type not modeled above (audio, reasoning-summary,
    /// code-interpreter, mcp, image-gen, refusal, annotation, ...).
    #[serde(other)]
    Unknown,
}

impl ResponseStreamEvent {
    /// This event's position in the stream, for resuming a background
    /// response via `retrieve_stream(id, starting_after)`. `None` only for
    /// [`ResponseStreamEvent::Unknown`] (payload not captured).
    pub fn sequence_number(&self) -> Option<u64> {
        match self {
            Self::Created { sequence_number, .. }
            | Self::InProgress { sequence_number, .. }
            | Self::Queued { sequence_number, .. }
            | Self::Completed { sequence_number, .. }
            | Self::Failed { sequence_number, .. }
            | Self::Incomplete { sequence_number, .. }
            | Self::OutputItemAdded { sequence_number, .. }
            | Self::OutputItemDone { sequence_number, .. }
            | Self::ContentPartAdded { sequence_number, .. }
            | Self::ContentPartDone { sequence_number, .. }
            | Self::OutputTextDelta { sequence_number, .. }
            | Self::OutputTextDone { sequence_number, .. }
            | Self::FunctionCallArgumentsDelta { sequence_number, .. }
            | Self::FunctionCallArgumentsDone { sequence_number, .. }
            | Self::Error { sequence_number, .. } => Some(*sequence_number),
            Self::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_json() -> serde_json::Value {
        serde_json::json!({
            "id": "resp_1", "object": "response", "created_at": 1.0,
            "model": "gpt-5", "status": "completed", "output": []
        })
    }

    #[test]
    fn stream_event_created() {
        let json = serde_json::json!({
            "type": "response.created", "response": response_json(), "sequence_number": 1
        });
        let event: ResponseStreamEvent = serde_json::from_value(json).unwrap();
        assert!(matches!(event, ResponseStreamEvent::Created { .. }));
        assert_eq!(event.sequence_number(), Some(1));
    }

    #[test]
    fn stream_event_output_text_delta() {
        let json = serde_json::json!({
            "type": "response.output_text.delta",
            "item_id": "msg_1", "output_index": 0, "content_index": 0, "delta": "Hel",
            "sequence_number": 3
        });
        let event: ResponseStreamEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.sequence_number(), Some(3));
        match event {
            ResponseStreamEvent::OutputTextDelta { delta, .. } => assert_eq!(delta, "Hel"),
            _ => panic!("expected OutputTextDelta"),
        }
    }

    #[test]
    fn unknown_stream_event_maps_to_unknown() {
        let json = serde_json::json!({
            "type": "response.reasoning_summary_text.delta",
            "item_id": "r_1", "delta": "thinking"
        });
        let event: ResponseStreamEvent = serde_json::from_value(json).unwrap();
        assert!(matches!(event, ResponseStreamEvent::Unknown));
        assert_eq!(event.sequence_number(), None);
    }

    #[test]
    fn failed_event_is_typed_not_error() {
        let mut response = response_json();
        response["status"] = serde_json::json!("failed");
        response["error"] = serde_json::json!({"code": "server_error", "message": "boom"});
        let json = serde_json::json!({
            "type": "response.failed", "response": response, "sequence_number": 5
        });
        let event: ResponseStreamEvent = serde_json::from_value(json).unwrap();
        match event {
            ResponseStreamEvent::Failed { response, .. } => {
                assert_eq!(response.error.unwrap().code, "server_error");
            }
            _ => panic!("expected a typed Ok(Failed), not a stream error"),
        }
    }

    #[test]
    fn error_event_uses_bare_type_tag() {
        let json = serde_json::json!({
            "type": "error", "message": "boom", "code": "server_error", "sequence_number": 2
        });
        let event: ResponseStreamEvent = serde_json::from_value(json).unwrap();
        match event {
            ResponseStreamEvent::Error { message, code, .. } => {
                assert_eq!(message, "boom");
                assert_eq!(code.as_deref(), Some("server_error"));
            }
            _ => panic!("expected Error"),
        }
    }
}
