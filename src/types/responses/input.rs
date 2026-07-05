//! The `input` polymorphism for Responses requests, mirroring
//! `openai-python/src/openai/types/responses/response_input_item_param.py`
//! (v1 subset: message / function_call / function_call_output / reasoning /
//! item_reference — exotic item types round-trip through `Other`).

use serde::{Deserialize, Serialize};

/// One part of a multimodal message content list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputContent {
    InputText {
        text: String,
    },
    InputImage {
        /// `"low" | "high" | "auto" | "original"`.
        detail: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image_url: Option<String>,
    },
    InputFile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file_url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file_data: Option<String>,
    },
}

impl InputContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self::InputText { text: text.into() }
    }

    pub fn image_url(url: impl Into<String>) -> Self {
        Self::InputImage {
            detail: "auto".to_string(),
            file_id: None,
            image_url: Some(url.into()),
        }
    }
}

/// `EasyInputMessage.content`: a plain string or a list of content parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EasyContent {
    Text(String),
    Parts(Vec<InputContent>),
}

impl From<&str> for EasyContent {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for EasyContent {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<InputContent>> for EasyContent {
    fn from(parts: Vec<InputContent>) -> Self {
        Self::Parts(parts)
    }
}

/// The typed input item variants this crate models directly. Anything else
/// (computer_use, shell, apply_patch, skills, MCP, compaction, ...) falls
/// back to [`InputItem::Other`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KnownInputItem {
    Message {
        role: String,
        content: EasyContent,
    },
    FunctionCall {
        call_id: String,
        name: String,
        /// JSON-encoded arguments string (may be invalid JSON — validate
        /// before use).
        arguments: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    FunctionCallOutput {
        call_id: String,
        /// Text output of the function call. Complex (image/file) outputs
        /// are not modeled in v1 — use [`InputItem::Other`] for those.
        output: String,
    },
    Reasoning {
        id: String,
        summary: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<Vec<serde_json::Value>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encrypted_content: Option<String>,
    },
    ItemReference {
        id: String,
    },
}

/// One item in the `input` list, mirroring `ResponseInputItemParam`.
///
/// Unrecognized `type` values deserialize into [`InputItem::Other`] and
/// round-trip without data loss (object key order is normalized through
/// `serde_json::Value`, but no field is dropped), so exotic item types
/// (computer_use, shell, apply_patch, skills, MCP, compaction, ...) survive
/// a decode/encode cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputItem {
    Known(KnownInputItem),
    Other(serde_json::Value),
}

impl InputItem {
    /// A user message with plain text or multimodal content.
    pub fn user(content: impl Into<EasyContent>) -> Self {
        Self::Known(KnownInputItem::Message {
            role: "user".to_string(),
            content: content.into(),
        })
    }

    /// A system/developer/assistant message with plain text or multimodal
    /// content.
    pub fn message(role: impl Into<String>, content: impl Into<EasyContent>) -> Self {
        Self::Known(KnownInputItem::Message {
            role: role.into(),
            content: content.into(),
        })
    }

    /// A function call previously made by the model (for replaying history).
    pub fn function_call(
        call_id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self::Known(KnownInputItem::FunctionCall {
            call_id: call_id.into(),
            name: name.into(),
            arguments: arguments.into(),
            id: None,
        })
    }

    /// The result of executing a function call, sent back to the model.
    pub fn function_call_output(call_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self::Known(KnownInputItem::FunctionCallOutput {
            call_id: call_id.into(),
            output: output.into(),
        })
    }

    /// A reference to a previously created item, by id.
    pub fn item_reference(id: impl Into<String>) -> Self {
        Self::Known(KnownInputItem::ItemReference { id: id.into() })
    }
}

/// The `input` request field: a plain string, or a list of input items.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum Input {
    #[default]
    Empty,
    Text(String),
    Items(Vec<InputItem>),
}

impl From<&str> for Input {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for Input {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<InputItem>> for Input {
    fn from(items: Vec<InputItem>) -> Self {
        Self::Items(items)
    }
}

impl Input {
    /// True for [`Input::Empty`] — used to omit `input` entirely from a
    /// continuation request driven purely by `previous_response_id`, rather
    /// than sending a JSON `null` the API rejects.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_input_serializes_as_string() {
        let input: Input = "hi".into();
        assert_eq!(serde_json::to_value(&input).unwrap(), "hi");
    }

    #[test]
    fn user_message_item_serializes() {
        let item = InputItem::user("hi");
        assert_eq!(
            serde_json::to_value(&item).unwrap(),
            serde_json::json!({"type": "message", "role": "user", "content": "hi"})
        );
    }

    #[test]
    fn function_call_output_roundtrip() {
        let item = InputItem::function_call_output("call_1", "72F and sunny");
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "function_call_output",
                "call_id": "call_1",
                "output": "72F and sunny"
            })
        );
        let roundtrip: InputItem = serde_json::from_value(json).unwrap();
        assert!(matches!(
            roundtrip,
            InputItem::Known(KnownInputItem::FunctionCallOutput { .. })
        ));
    }

    #[test]
    fn unknown_input_item_preserved_as_other() {
        let json = serde_json::json!({
            "type": "computer_call",
            "call_id": "call_1",
            "action": {"type": "click", "x": 1, "y": 2}
        });
        let item: InputItem = serde_json::from_value(json.clone()).unwrap();
        assert!(matches!(item, InputItem::Other(_)));
        assert_eq!(serde_json::to_value(&item).unwrap(), json);
    }

    #[test]
    fn multimodal_content_serializes() {
        let item = InputItem::user(vec![
            InputContent::text("What is this?"),
            InputContent::image_url("https://example.com/cat.png"),
        ]);
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["content"][0]["type"], "input_text");
        assert_eq!(json["content"][1]["type"], "input_image");
    }
}
