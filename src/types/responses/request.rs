//! `CreateResponseRequest`, mirroring the `create` parameter list in
//! `resources/responses/responses.py`.

use std::collections::HashMap;

use serde::Serialize;

use super::config::{Reasoning, ResponseIncludable, ResponseTextConfig, Truncation};
use super::input::Input;
use super::tools::{Tool, ToolChoice};

/// Prompt-cache options for a Responses request (`gpt-5.6` and later).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ResponsesPromptCacheOptions {
    /// `"implicit"` (default) or `"explicit"`; `explicit` disables the
    /// automatic breakpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Minimum cache lifetime; currently only `"30m"` (the default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
    /// Prepare the cache without generating output (forces `generate` off).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prewarm: Option<bool>,
    /// Response to compare against when diagnosing prompt cache reuse;
    /// diagnostics appear in `Response.extra["prompt_cache_diagnostics"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison_response_id: Option<String>,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_options: Option<ResponsesPromptCacheOptions>,
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

    pub fn prompt_cache_options(mut self, options: ResponsesPromptCacheOptions) -> Self {
        self.prompt_cache_options = Some(options);
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

/// Request body for `POST /responses/compact`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CompactResponseRequest {
    pub model: String,
    #[serde(skip_serializing_if = "Input::is_empty")]
    pub input: Input,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_options: Option<ResponsesPromptCacheOptions>,
    /// `"in_memory"` or `"24h"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<String>,
    /// `"auto"`, `"default"`, `"fast"`, `"flex"` or `"priority"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// Additional request parameters flattened into the JSON body.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CompactResponseRequest {
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

    pub fn previous_response_id(mut self, id: impl Into<String>) -> Self {
        self.previous_response_id = Some(id.into());
        self
    }

    pub fn prompt_cache_key(mut self, key: impl Into<String>) -> Self {
        self.prompt_cache_key = Some(key.into());
        self
    }

    pub fn prompt_cache_options(mut self, options: ResponsesPromptCacheOptions) -> Self {
        self.prompt_cache_options = Some(options);
        self
    }

    pub fn prompt_cache_retention(mut self, retention: impl Into<String>) -> Self {
        self.prompt_cache_retention = Some(retention.into());
        self
    }

    pub fn service_tier(mut self, tier: impl Into<String>) -> Self {
        self.service_tier = Some(tier.into());
        self
    }

    /// Set an arbitrary extra parameter on the request body.
    pub fn param(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

/// Request body for `POST /responses/input_tokens`: every field is optional
/// and describes the request whose input tokens should be counted.
#[derive(Debug, Clone, Default, Serialize)]
pub struct InputTokenCountRequest {
    /// Conversation id whose items are prepended to `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    #[serde(skip_serializing_if = "Input::is_empty")]
    pub input: Input,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Model-owned style preset (e.g. `"friendly"`, `"pragmatic"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<Truncation>,
    /// Additional request parameters flattened into the JSON body.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl InputTokenCountRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn input(mut self, input: impl Into<Input>) -> Self {
        self.input = input.into();
        self
    }

    pub fn conversation(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation = Some(conversation_id.into());
        self
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn parallel_tool_calls(mut self, parallel: bool) -> Self {
        self.parallel_tool_calls = Some(parallel);
        self
    }

    pub fn personality(mut self, personality: impl Into<String>) -> Self {
        self.personality = Some(personality.into());
        self
    }

    pub fn previous_response_id(mut self, id: impl Into<String>) -> Self {
        self.previous_response_id = Some(id.into());
        self
    }

    pub fn reasoning(mut self, reasoning: Reasoning) -> Self {
        self.reasoning = Some(reasoning);
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

    pub fn truncation(mut self, truncation: Truncation) -> Self {
        self.truncation = Some(truncation);
        self
    }

    /// Set an arbitrary extra parameter on the request body.
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
    fn prompt_cache_options_serialize_all_fields_and_omit_none() {
        let request = CreateResponseRequest::new("gpt-5", "hi").prompt_cache_options(
            ResponsesPromptCacheOptions {
                mode: Some("explicit".into()),
                ttl: Some("30m".into()),
                prewarm: Some(true),
                comparison_response_id: Some("resp_0".into()),
            },
        );
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json["prompt_cache_options"],
            serde_json::json!({
                "mode": "explicit",
                "ttl": "30m",
                "prewarm": true,
                "comparison_response_id": "resp_0"
            })
        );

        let empty = CreateResponseRequest::new("gpt-5", "hi")
            .prompt_cache_options(ResponsesPromptCacheOptions::default());
        assert_eq!(
            serde_json::to_value(&empty).unwrap()["prompt_cache_options"],
            serde_json::json!({})
        );
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
