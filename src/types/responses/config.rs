//! Shared Responses config types: `text.format`, `reasoning`, `include`,
//! `truncation`. Mirrors `openai-python/src/openai/types/responses/`
//! (`response_text_config.py`, `response_format_text_config.py`,
//! `reasoning.py`, `response_includable.py`).

use serde::{Deserialize, Serialize};

/// Reasoning effort for reasoning models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

/// Reasoning configuration for reasoning models (`o1`/`o3`/`gpt-5`-class).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Reasoning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffort>,
    /// `"auto" | "concise" | "detailed"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Which prior reasoning to keep in context:
    /// `"auto" | "current_turn" | "all_turns"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Reasoning mode: `"standard" | "pro"` (open set upstream).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl Reasoning {
    pub fn effort(effort: ReasoningEffort) -> Self {
        Self {
            effort: Some(effort),
            ..Self::default()
        }
    }

    /// `"auto" | "concise" | "detailed"`.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// `"auto" | "current_turn" | "all_turns"`.
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// `"standard" | "pro"`.
    pub fn mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = Some(mode.into());
        self
    }
}

/// The `text.format` structured-output configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormatTextConfig {
    Text,
    JsonObject,
    JsonSchema {
        name: String,
        schema: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        strict: Option<bool>,
    },
}

impl ResponseFormatTextConfig {
    pub fn json_schema(name: impl Into<String>, schema: serde_json::Value, strict: bool) -> Self {
        Self::JsonSchema {
            name: name.into(),
            schema,
            description: None,
            strict: Some(strict),
        }
    }
}

/// `text` request field: output format + verbosity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseTextConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<ResponseFormatTextConfig>,
    /// `"low" | "medium" | "high"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<String>,
}

impl ResponseTextConfig {
    pub fn format(format: ResponseFormatTextConfig) -> Self {
        Self {
            format: Some(format),
            verbosity: None,
        }
    }
}

/// Additional output data to include (`include` request param).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseIncludable {
    #[serde(rename = "file_search_call.results")]
    FileSearchCallResults,
    #[serde(rename = "web_search_call.results")]
    WebSearchCallResults,
    #[serde(rename = "web_search_call.action.sources")]
    WebSearchCallActionSources,
    #[serde(rename = "message.input_image.image_url")]
    MessageInputImageImageUrl,
    #[serde(rename = "computer_call_output.output.image_url")]
    ComputerCallOutputOutputImageUrl,
    #[serde(rename = "code_interpreter_call.outputs")]
    CodeInterpreterCallOutputs,
    #[serde(rename = "reasoning.encrypted_content")]
    ReasoningEncryptedContent,
    #[serde(rename = "message.output_text.logprobs")]
    MessageOutputTextLogprobs,
}

impl ResponseIncludable {
    /// The wire value, for building repeated `include[]` query parameters.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileSearchCallResults => "file_search_call.results",
            Self::WebSearchCallResults => "web_search_call.results",
            Self::WebSearchCallActionSources => "web_search_call.action.sources",
            Self::MessageInputImageImageUrl => "message.input_image.image_url",
            Self::ComputerCallOutputOutputImageUrl => "computer_call_output.output.image_url",
            Self::CodeInterpreterCallOutputs => "code_interpreter_call.outputs",
            Self::ReasoningEncryptedContent => "reasoning.encrypted_content",
            Self::MessageOutputTextLogprobs => "message.output_text.logprobs",
        }
    }
}

/// Context-window truncation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Truncation {
    Auto,
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasoning_serializes() {
        let reasoning = Reasoning::effort(ReasoningEffort::Medium);
        assert_eq!(
            serde_json::to_value(&reasoning).unwrap(),
            serde_json::json!({"effort": "medium"})
        );
    }

    #[test]
    fn reasoning_effort_max_serializes() {
        assert_eq!(
            serde_json::to_value(ReasoningEffort::Max).unwrap(),
            serde_json::json!("max")
        );
    }

    #[test]
    fn reasoning_setters_serialize_context_and_mode() {
        let reasoning = Reasoning::effort(ReasoningEffort::High)
            .summary("auto")
            .context("all_turns")
            .mode("pro");
        assert_eq!(
            serde_json::to_value(&reasoning).unwrap(),
            serde_json::json!({
                "effort": "high",
                "summary": "auto",
                "context": "all_turns",
                "mode": "pro"
            })
        );
    }

    #[test]
    fn text_json_schema_serializes() {
        let text = ResponseTextConfig::format(ResponseFormatTextConfig::json_schema(
            "out",
            serde_json::json!({"type": "object"}),
            true,
        ));
        assert_eq!(
            serde_json::to_value(&text).unwrap(),
            serde_json::json!({
                "format": {
                    "type": "json_schema",
                    "name": "out",
                    "schema": {"type": "object"},
                    "strict": true
                }
            })
        );
    }

    #[test]
    fn includable_uses_dotted_wire_values() {
        assert_eq!(
            serde_json::to_value(ResponseIncludable::FileSearchCallResults).unwrap(),
            "file_search_call.results"
        );
    }
}
