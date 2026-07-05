//! `CreateResponseRequest`, mirroring the `create` parameter list in
//! `resources/responses/responses.py`.

use std::collections::HashMap;

use serde::Serialize;

use super::config::{Reasoning, ResponseIncludable, ResponseTextConfig, Truncation};
use super::input::Input;
use super::tools::{Tool, ToolChoice};

/// Request body for `POST /responses`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateResponseRequest {
    pub model: String,
    /// Omitted from the request body when empty (e.g. a continuation request
    /// driven purely by `previous_response_id`) rather than sent as `null`.
    #[serde(skip_serializing_if = "Input::is_empty")]
    pub input: Input,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tool_calls: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<Truncation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<ResponseIncludable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Escape hatch: additional request parameters flattened into the JSON
    /// body, for provider-specific or newly-added params not yet typed.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CreateResponseRequest {
    pub fn new(model: impl Into<String>, input: impl Into<Input>) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            ..Self::default()
        }
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn max_output_tokens(mut self, max: u64) -> Self {
        self.max_output_tokens = Some(max);
        self
    }

    pub fn max_tool_calls(mut self, max: u64) -> Self {
        self.max_tool_calls = Some(max);
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Chain this request onto a prior response, for stateful multi-turn
    /// tool-calling loops.
    pub fn previous_response_id(mut self, id: impl Into<String>) -> Self {
        self.previous_response_id = Some(id.into());
        self
    }

    pub fn reasoning(mut self, reasoning: Reasoning) -> Self {
        self.reasoning = Some(reasoning);
        self
    }

    pub fn store(mut self, store: bool) -> Self {
        self.store = Some(store);
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn text(mut self, text: ResponseTextConfig) -> Self {
        self.text = Some(text);
        self
    }

    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn truncation(mut self, truncation: Truncation) -> Self {
        self.truncation = Some(truncation);
        self
    }

    pub fn include(mut self, include: Vec<ResponseIncludable>) -> Self {
        self.include = Some(include);
        self
    }

    /// Run this response asynchronously; poll or stream-resume via
    /// `retrieve`/`retrieve_stream`.
    pub fn background(mut self, background: bool) -> Self {
        self.background = Some(background);
        self
    }

    pub fn parallel_tool_calls(mut self, parallel: bool) -> Self {
        self.parallel_tool_calls = Some(parallel);
        self
    }

    /// Set an arbitrary extra parameter on the request body (escape hatch
    /// for provider-specific or newly-added params). Do not use this to set
    /// `stream`: `extra` is flattened after the typed fields, so a
    /// `param("stream", ...)` value serializes last and silently overrides
    /// whatever `create`/`create_stream` set on the typed `stream` field.
    pub fn param(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::input::InputItem;
    use super::*;

    #[test]
    fn request_skips_none_fields() {
        let request = CreateResponseRequest::new("gpt-5", "hi");
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json, serde_json::json!({"model": "gpt-5", "input": "hi"}));
    }

    #[test]
    fn request_with_items_input_serializes() {
        let request =
            CreateResponseRequest::new("gpt-5", vec![InputItem::user("hi")]).temperature(0.5);
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "gpt-5");
        assert_eq!(json["temperature"], 0.5);
        assert_eq!(json["input"][0]["type"], "message");
    }

    #[test]
    fn continuation_request_omits_input_entirely() {
        // A follow-up request driven purely by `previous_response_id` has no
        // `input` to send; it must be omitted, not serialized as `null`.
        let request = CreateResponseRequest {
            model: "gpt-5".to_string(),
            ..Default::default()
        }
        .previous_response_id("resp_1");
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"model": "gpt-5", "previous_response_id": "resp_1"})
        );
    }
}
