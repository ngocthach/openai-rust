//! Assistants (beta v2) types, mirroring `openai-python/src/openai/types/beta/`.
//!
//! Response types are typed at the top level; deeply polymorphic fields
//! (tools, tool resources, tool choice, response format, required actions,
//! step details, …) are kept as [`serde_json::Value`] to bound scope. Only
//! the `text` message-content block and [`RunStatus`] are modelled as enums.
//!
//! Streaming runs are intentionally out of scope for this module.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pagination::HasId;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// An assistant, mirroring `types/beta/assistant.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Assistant {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub tools: Vec<Value>,
    #[serde(default)]
    pub response_format: Option<Value>,
    #[serde(default)]
    pub tool_resources: Option<Value>,
}

impl HasId for Assistant {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// A thread, mirroring `types/beta/thread.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Thread {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub tool_resources: Option<Value>,
}

impl HasId for Thread {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// The `text` payload inside a [`MessageContentBlock::Text`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageText {
    pub value: String,
    #[serde(default)]
    pub annotations: Vec<Value>,
}

/// A block of message content. Only the `text` block is modelled; every other
/// block type (`image_file`, `image_url`, `refusal`, …) falls through to
/// [`MessageContentBlock::Other`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum MessageContentBlock {
    /// A block of generated text.
    Text { text: MessageText },
    /// Any non-text block (kept minimal on purpose).
    #[serde(other)]
    Other,
}

/// A message on a thread, mirroring `types/beta/threads/message.py`.
///
/// Named `ThreadMessage` to avoid colliding with the chat-completions
/// `Message` request type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ThreadMessage {
    pub id: String,
    #[serde(default)]
    pub thread_id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub content: Vec<MessageContentBlock>,
    #[serde(default)]
    pub assistant_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub attachments: Option<Vec<Value>>,
    #[serde(default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub incomplete_at: Option<i64>,
    #[serde(default)]
    pub incomplete_details: Option<Value>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl HasId for ThreadMessage {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

impl ThreadMessage {
    /// Extract the concatenated text of the message, if any, by reading the
    /// first `text` content block's `value`.
    pub fn text(&self) -> Option<String> {
        self.content.iter().find_map(|block| match block {
            MessageContentBlock::Text { text } => Some(text.value.clone()),
            MessageContentBlock::Other => None,
        })
    }
}

/// The lifecycle status of a [`Run`], mirroring
/// `types/beta/threads/run_status.py`. Unknown values deserialize to
/// [`RunStatus::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RunStatus {
    Queued,
    InProgress,
    RequiresAction,
    Cancelling,
    Cancelled,
    Failed,
    Completed,
    Incomplete,
    Expired,
    /// A status not recognised by this client version.
    #[serde(other)]
    Unknown,
}

/// Token usage for a completed run or run step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunUsage {
    #[serde(default)]
    pub completion_tokens: i64,
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
}

/// A run of an assistant on a thread, mirroring `types/beta/threads/run.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Run {
    pub id: String,
    #[serde(default)]
    pub thread_id: String,
    #[serde(default)]
    pub assistant_id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub object: String,
    #[serde(default = "run_status_unknown")]
    pub status: RunStatus,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub parallel_tool_calls: bool,
    #[serde(default)]
    pub tools: Vec<Value>,
    #[serde(default)]
    pub started_at: Option<i64>,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub cancelled_at: Option<i64>,
    #[serde(default)]
    pub failed_at: Option<i64>,
    #[serde(default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub last_error: Option<Value>,
    #[serde(default)]
    pub incomplete_details: Option<Value>,
    #[serde(default)]
    pub required_action: Option<Value>,
    #[serde(default)]
    pub response_format: Option<Value>,
    #[serde(default)]
    pub tool_choice: Option<Value>,
    #[serde(default)]
    pub truncation_strategy: Option<Value>,
    #[serde(default)]
    pub usage: Option<RunUsage>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub max_completion_tokens: Option<i64>,
    #[serde(default)]
    pub max_prompt_tokens: Option<i64>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

fn run_status_unknown() -> RunStatus {
    RunStatus::Unknown
}

impl HasId for Run {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// A step taken during a run, mirroring `types/beta/threads/runs/run_step.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunStep {
    pub id: String,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub thread_id: String,
    #[serde(default)]
    pub assistant_id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub object: String,
    #[serde(rename = "type", default)]
    pub step_type: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub step_details: Value,
    #[serde(default)]
    pub last_error: Option<Value>,
    #[serde(default)]
    pub usage: Option<RunUsage>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub expired_at: Option<i64>,
    #[serde(default)]
    pub cancelled_at: Option<i64>,
    #[serde(default)]
    pub failed_at: Option<i64>,
    #[serde(default)]
    pub completed_at: Option<i64>,
}

impl HasId for RunStep {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// Response from `DELETE /assistants/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssistantDeleted {
    pub id: String,
    pub deleted: bool,
    #[serde(default)]
    pub object: String,
}

/// Response from `DELETE /threads/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ThreadDeleted {
    pub id: String,
    pub deleted: bool,
    #[serde(default)]
    pub object: String,
}

/// Response from `DELETE /threads/{tid}/messages/{mid}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageDeleted {
    pub id: String,
    pub deleted: bool,
    #[serde(default)]
    pub object: String,
}

