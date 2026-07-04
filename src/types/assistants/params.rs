//! Assistants (beta v2) request parameter types.
//!
//! Scalars are typed; deeply polymorphic fields use [`serde_json::Value`].

use serde::Serialize;
use serde_json::Value;

// ---------------------------------------------------------------------------
// List query parameters (shared across the beta list endpoints)
// ---------------------------------------------------------------------------

/// Query parameters for the beta cursor-paginated list endpoints.
///
/// Not every field applies to every endpoint (`run_id` is messages-only,
/// `include` is steps-only); unused fields are simply left unset (DRY).
#[derive(Debug, Clone, Default)]
pub struct ListParams {
    pub after: Option<String>,
    pub before: Option<String>,
    pub limit: Option<u32>,
    pub order: Option<String>,
    pub run_id: Option<String>,
    pub include: Option<Vec<String>>,
}

impl ListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    pub fn before(mut self, before: impl Into<String>) -> Self {
        self.before = Some(before.into());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sort order: `"asc"` or `"desc"`.
    pub fn order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    /// Filter messages by originating run (messages list only).
    pub fn run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    /// Extra fields to include in the response (run steps list only).
    pub fn include(mut self, include: Vec<String>) -> Self {
        self.include = Some(include);
        self
    }

    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        let mut query = Vec::new();
        if let Some(after) = &self.after {
            query.push(("after".into(), after.clone()));
        }
        if let Some(before) = &self.before {
            query.push(("before".into(), before.clone()));
        }
        if let Some(limit) = self.limit {
            query.push(("limit".into(), limit.to_string()));
        }
        if let Some(order) = &self.order {
            query.push(("order".into(), order.clone()));
        }
        if let Some(run_id) = &self.run_id {
            query.push(("run_id".into(), run_id.clone()));
        }
        if let Some(include) = &self.include {
            for item in include {
                query.push(("include[]".into(), item.clone()));
            }
        }
        query
    }
}

// ---------------------------------------------------------------------------
// Assistant params
// ---------------------------------------------------------------------------

/// Request body for `POST /assistants`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AssistantCreateParams {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<Value>,
}

impl AssistantCreateParams {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Self::default()
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    pub fn response_format(mut self, response_format: Value) -> Self {
        self.response_format = Some(response_format);
        self
    }

    pub fn tools(mut self, tools: Vec<Value>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn tool_resources(mut self, tool_resources: Value) -> Self {
        self.tool_resources = Some(tool_resources);
        self
    }
}

/// Request body for `POST /assistants/{id}` (update). All fields optional.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AssistantUpdateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<Value>,
}

impl AssistantUpdateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn tools(mut self, tools: Vec<Value>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn tool_resources(mut self, tool_resources: Value) -> Self {
        self.tool_resources = Some(tool_resources);
        self
    }
}

// ---------------------------------------------------------------------------
// Thread params
// ---------------------------------------------------------------------------

/// Request body for `POST /threads`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ThreadCreateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<Value>,
}

impl ThreadCreateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn messages(mut self, messages: Vec<Value>) -> Self {
        self.messages = Some(messages);
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn tool_resources(mut self, tool_resources: Value) -> Self {
        self.tool_resources = Some(tool_resources);
        self
    }
}

/// Request body for `POST /threads/{id}` (update).
#[derive(Debug, Clone, Serialize, Default)]
pub struct ThreadUpdateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<Value>,
}

impl ThreadUpdateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn tool_resources(mut self, tool_resources: Value) -> Self {
        self.tool_resources = Some(tool_resources);
        self
    }
}

// ---------------------------------------------------------------------------
// Message params
// ---------------------------------------------------------------------------

/// Message content on create: a plain string or a list of content blocks
/// (kept as [`serde_json::Value`] for the polymorphic block shapes).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MessageContentInput {
    Text(String),
    Blocks(Vec<Value>),
}

impl From<&str> for MessageContentInput {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for MessageContentInput {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<Value>> for MessageContentInput {
    fn from(blocks: Vec<Value>) -> Self {
        Self::Blocks(blocks)
    }
}

/// Request body for `POST /threads/{tid}/messages`.
#[derive(Debug, Clone, Serialize)]
pub struct MessageCreateParams {
    /// `"user"` or `"assistant"`.
    pub role: String,
    pub content: MessageContentInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl MessageCreateParams {
    pub fn new(role: impl Into<String>, content: impl Into<MessageContentInput>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            attachments: None,
            metadata: None,
        }
    }

    /// A message with role `"user"`.
    pub fn user(content: impl Into<MessageContentInput>) -> Self {
        Self::new("user", content)
    }

    /// A message with role `"assistant"`.
    pub fn assistant(content: impl Into<MessageContentInput>) -> Self {
        Self::new("assistant", content)
    }

    pub fn attachments(mut self, attachments: Vec<Value>) -> Self {
        self.attachments = Some(attachments);
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Request body for `POST /threads/{tid}/messages/{mid}` (update).
#[derive(Debug, Clone, Serialize, Default)]
pub struct MessageUpdateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl MessageUpdateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

// ---------------------------------------------------------------------------
// Run params
// ---------------------------------------------------------------------------

/// Request body for `POST /threads/{tid}/runs`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RunCreateParams {
    pub assistant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_messages: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_prompt_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_strategy: Option<Value>,
}

impl RunCreateParams {
    pub fn new(assistant_id: impl Into<String>) -> Self {
        Self {
            assistant_id: assistant_id.into(),
            ..Self::default()
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn additional_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.additional_instructions = Some(instructions.into());
        self
    }

    pub fn additional_messages(mut self, messages: Vec<Value>) -> Self {
        self.additional_messages = Some(messages);
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn max_completion_tokens(mut self, max: i64) -> Self {
        self.max_completion_tokens = Some(max);
        self
    }

    pub fn max_prompt_tokens(mut self, max: i64) -> Self {
        self.max_prompt_tokens = Some(max);
        self
    }

    pub fn parallel_tool_calls(mut self, parallel: bool) -> Self {
        self.parallel_tool_calls = Some(parallel);
        self
    }

    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    pub fn response_format(mut self, response_format: Value) -> Self {
        self.response_format = Some(response_format);
        self
    }

    pub fn tool_choice(mut self, tool_choice: Value) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    pub fn tools(mut self, tools: Vec<Value>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn truncation_strategy(mut self, strategy: Value) -> Self {
        self.truncation_strategy = Some(strategy);
        self
    }
}

/// Request body for `POST /threads/{tid}/runs/{rid}` (update).
#[derive(Debug, Clone, Serialize, Default)]
pub struct RunUpdateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl RunUpdateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// A single tool output submitted to a run awaiting action.
#[derive(Debug, Clone, Serialize)]
pub struct ToolOutput {
    pub tool_call_id: String,
    pub output: String,
}

impl ToolOutput {
    pub fn new(tool_call_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            output: output.into(),
        }
    }
}

/// Request body for `POST /threads/{tid}/runs/{rid}/submit_tool_outputs`.
#[derive(Debug, Clone, Serialize)]
pub struct SubmitToolOutputsParams {
    pub tool_outputs: Vec<ToolOutput>,
}

impl SubmitToolOutputsParams {
    pub fn new(tool_outputs: Vec<ToolOutput>) -> Self {
        Self { tool_outputs }
    }
}
