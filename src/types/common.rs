//! Shared API types: roles, finish reasons, usage, response formats, tools.
//! Mirrors `openai-python/src/openai/types/` shared definitions.

use serde::{Deserialize, Serialize};

/// Message author role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
    Developer,
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
    /// Forward-compatibility with providers returning nonstandard values.
    #[serde(other)]
    Unknown,
}

/// Token usage statistics, from `completion_usage.py`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    /// Provider-specific breakdown (cached tokens, audio tokens, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<serde_json::Value>,
    /// Provider-specific breakdown (reasoning tokens, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<serde_json::Value>,
}

/// File-expiration policy used by uploads (`expires_after`) and batch output
/// files (`output_expires_after`): expire `seconds` after `anchor`
/// (`"created_at"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiresAfter {
    pub anchor: String,
    pub seconds: u64,
}

impl ExpiresAfter {
    /// Expire `seconds` after creation (anchor `"created_at"`).
    pub fn after_creation(seconds: u64) -> Self {
        Self {
            anchor: "created_at".into(),
            seconds,
        }
    }
}

/// Output format constraint for chat completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    JsonObject,
    /// Structured outputs: pass the full `json_schema` object
    /// (`{"name": ..., "schema": {...}, "strict": ...}`).
    JsonSchema { json_schema: serde_json::Value },
}

impl ResponseFormat {
    /// Convenience constructor for `json_schema` format.
    pub fn json_schema(name: impl Into<String>, schema: serde_json::Value, strict: bool) -> Self {
        Self::JsonSchema {
            json_schema: serde_json::json!({
                "name": name.into(),
                "schema": schema,
                "strict": strict,
            }),
        }
    }
}

/// A tool the model may call. Only `function` tools are supported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDef,
}

impl Tool {
    /// A `function` tool with a JSON-schema `parameters` object.
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            tool_type: "function".into(),
            function: FunctionDef {
                name: name.into(),
                description: Some(description.into()),
                parameters: Some(parameters),
                strict: None,
            },
        }
    }
}

/// Function definition inside a [`Tool`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// How the model chooses tools: `"none" | "auto" | "required"` or a specific
/// function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolChoice {
    None,
    Auto,
    Required,
    /// Force a call to the named function.
    Function(String),
}

impl Serialize for ToolChoice {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::None => serializer.serialize_str("none"),
            Self::Auto => serializer.serialize_str("auto"),
            Self::Required => serializer.serialize_str("required"),
            Self::Function(name) => serde_json::json!({
                "type": "function",
                "function": { "name": name },
            })
            .serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_choice_serialization() {
        assert_eq!(serde_json::to_value(ToolChoice::Auto).unwrap(), "auto");
        assert_eq!(serde_json::to_value(ToolChoice::None).unwrap(), "none");
        assert_eq!(
            serde_json::to_value(ToolChoice::Function("get_weather".into())).unwrap(),
            serde_json::json!({"type": "function", "function": {"name": "get_weather"}})
        );
    }

    #[test]
    fn response_format_tagging() {
        assert_eq!(
            serde_json::to_value(ResponseFormat::JsonObject).unwrap(),
            serde_json::json!({"type": "json_object"})
        );
        let schema = ResponseFormat::json_schema("out", serde_json::json!({"type": "object"}), true);
        assert_eq!(
            serde_json::to_value(schema).unwrap(),
            serde_json::json!({
                "type": "json_schema",
                "json_schema": {"name": "out", "schema": {"type": "object"}, "strict": true}
            })
        );
    }

    #[test]
    fn unknown_finish_reason_is_forward_compatible() {
        let reason: FinishReason = serde_json::from_str("\"eos_token\"").unwrap();
        assert_eq!(reason, FinishReason::Unknown);
    }
}
