//! The `tools` / `tool_choice` polymorphism for Responses requests, mirroring
//! `openai-python/src/openai/types/responses/tool_param.py` (v1 subset:
//! function + thin web_search/file_search/code_interpreter — everything else
//! round-trips through `Tool::Other`).

use serde::{Deserialize, Serialize};

/// A function the model may call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// The typed tool variants this crate models directly. Built-ins beyond
/// `web_search`/`file_search`/`code_interpreter` (`mcp`, `computer_use_preview`,
/// `image_generation`, `local_shell`, `custom`, ...) fall back to
/// [`Tool::Other`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KnownTool {
    Function(FunctionTool),
    WebSearch {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_context_size: Option<String>,
    },
    FileSearch {
        vector_store_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_num_results: Option<u32>,
    },
    CodeInterpreter {
        /// `"auto"` or `{"type": "auto", "file_ids": [...]}` shape from the
        /// Python SDK; kept untyped to avoid over-modeling a rarely-varied
        /// field.
        container: serde_json::Value,
    },
}

/// A tool the model may call, mirroring `ToolParam`.
///
/// Unrecognized `type` values deserialize into [`Tool::Other`] and round-trip
/// without data loss (object key order is normalized through
/// `serde_json::Value`, but no field is dropped).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tool {
    Known(KnownTool),
    Other(serde_json::Value),
}

impl Tool {
    /// A `function` tool with a JSON-schema `parameters` object.
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self::Known(KnownTool::Function(FunctionTool {
            name: name.into(),
            description: Some(description.into()),
            parameters: Some(parameters),
            strict: None,
        }))
    }

    /// The built-in web search tool.
    pub fn web_search() -> Self {
        Self::Known(KnownTool::WebSearch {
            search_context_size: None,
        })
    }

    /// The built-in file search tool over the given vector stores.
    pub fn file_search(vector_store_ids: Vec<String>) -> Self {
        Self::Known(KnownTool::FileSearch {
            vector_store_ids,
            max_num_results: None,
        })
    }
}

/// How the model chooses tools: `"none" | "auto" | "required"`, or a
/// provider-specific typed choice (e.g. forcing a named function).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Mode(String),
    Typed(serde_json::Value),
}

impl ToolChoice {
    pub const NONE: &'static str = "none";
    pub const AUTO: &'static str = "auto";
    pub const REQUIRED: &'static str = "required";

    pub fn mode(mode: impl Into<String>) -> Self {
        Self::Mode(mode.into())
    }

    /// Force a call to the named function.
    pub fn function(name: impl Into<String>) -> Self {
        Self::Typed(serde_json::json!({
            "type": "function",
            "name": name.into(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_tool_serializes() {
        let tool = Tool::function(
            "get_weather",
            "Get current weather",
            serde_json::json!({"type": "object"}),
        );
        assert_eq!(
            serde_json::to_value(&tool).unwrap(),
            serde_json::json!({
                "type": "function",
                "name": "get_weather",
                "description": "Get current weather",
                "parameters": {"type": "object"}
            })
        );
    }

    #[test]
    fn unknown_tool_preserved() {
        let json = serde_json::json!({"type": "mcp", "server_label": "s", "server_url": "u"});
        let tool: Tool = serde_json::from_value(json.clone()).unwrap();
        assert!(matches!(tool, Tool::Other(_)));
        assert_eq!(serde_json::to_value(&tool).unwrap(), json);
    }

    #[test]
    fn tool_choice_mode_serializes_as_plain_string() {
        assert_eq!(
            serde_json::to_value(ToolChoice::mode("auto")).unwrap(),
            "auto"
        );
    }
}
