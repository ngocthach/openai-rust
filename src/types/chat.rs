//! Chat completions request/response/chunk types, mirroring
//! `openai-python/src/openai/types/chat/`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::common::{
    lenient_string_map, FinishReason, ResponseFormat, Role, Tool, ToolChoice, Usage,
};
use crate::pagination::{cursor_query, HasId};
use crate::request::bracket_query;

/// Message content: a plain string or a list of multimodal parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl From<&str> for MessageContent {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for MessageContent {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<ContentPart>> for MessageContent {
    fn from(parts: Vec<ContentPart>) -> Self {
        Self::Parts(parts)
    }
}

/// One part of a multimodal message, mirroring
/// `types/chat/chat_completion_content_part_param.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
    InputAudio { input_audio: InputAudio },
}

impl ContentPart {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// An image by URL or `data:` URI.
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::ImageUrl {
            image_url: ImageUrl {
                url: url.into(),
                detail: None,
            },
        }
    }

    /// An image with an explicit detail level (`"auto"`, `"low"`, `"high"`).
    pub fn image_url_with_detail(url: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::ImageUrl {
            image_url: ImageUrl {
                url: url.into(),
                detail: Some(detail.into()),
            },
        }
    }

    /// Base64-encoded audio (`format`: `"wav"` or `"mp3"`).
    pub fn input_audio(data: impl Into<String>, format: impl Into<String>) -> Self {
        Self::InputAudio {
            input_audio: InputAudio {
                data: data.into(),
                format: format.into(),
            },
        }
    }
}

/// Image reference inside a [`ContentPart::ImageUrl`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Audio payload inside a [`ContentPart::InputAudio`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudio {
    /// Base64-encoded audio data.
    pub data: String,
    pub format: String,
}

/// A chat message sent in a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool calls previously made by the assistant (assistant messages only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// The tool call this message responds to (tool messages only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    fn new(role: Role, content: impl Into<MessageContent>) -> Self {
        Self {
            role,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn system(content: impl Into<MessageContent>) -> Self {
        Self::new(Role::System, content)
    }

    pub fn developer(content: impl Into<MessageContent>) -> Self {
        Self::new(Role::Developer, content)
    }

    /// A user message: pass a string, or a `Vec<ContentPart>` for multimodal
    /// content (text + images + audio).
    pub fn user(content: impl Into<MessageContent>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<MessageContent>) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// An assistant message carrying tool calls (for replaying history).
    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: None,
            name: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// A tool result message answering `tool_call_id`.
    pub fn tool(content: impl Into<MessageContent>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content.into()),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// `stop` accepts a single string or up to 4 strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    One(String),
    Many(Vec<String>),
}

/// Options for streaming responses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

/// Prompt-cache options for a chat completion request (`gpt-5.6` and later).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChatPromptCacheOptions {
    /// `"implicit"` (default) or `"explicit"`; `explicit` disables the
    /// automatic breakpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Minimum cache lifetime; currently only `"30m"` (the default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

/// Request body for `POST /chat/completions`, mirroring the parameter list in
/// `resources/chat/completions/completions.py::create`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    /// Deprecated upstream in favor of `max_completion_tokens`, but still
    /// widely supported by OpenAI-compatible providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Reasoning effort for reasoning models (`"minimal"`, `"low"`,
    /// `"medium"`, `"high"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<std::collections::HashMap<String, i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// Output modalities, e.g. `["text", "audio"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,
    /// Audio output configuration (`{"voice": ..., "format": ...}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_options: Option<ChatPromptCacheOptions>,
    /// Escape hatch: additional request parameters flattened into the JSON
    /// body, for provider-specific or newly-added params not yet typed.
    #[serde(flatten, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl ChatCompletionRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            ..Self::default()
        }
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn max_completion_tokens(mut self, max: u64) -> Self {
        self.max_completion_tokens = Some(max);
        self
    }

    pub fn max_tokens(mut self, max: u64) -> Self {
        self.max_tokens = Some(max);
        self
    }

    pub fn top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn n(mut self, n: u32) -> Self {
        self.n = Some(n);
        self
    }

    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn frequency_penalty(mut self, penalty: f64) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    pub fn presence_penalty(mut self, penalty: f64) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    pub fn logprobs(mut self, logprobs: bool) -> Self {
        self.logprobs = Some(logprobs);
        self
    }

    pub fn prompt_cache_options(mut self, options: ChatPromptCacheOptions) -> Self {
        self.prompt_cache_options = Some(options);
        self
    }

    pub fn top_logprobs(mut self, top_logprobs: u32) -> Self {
        self.top_logprobs = Some(top_logprobs);
        self
    }

    pub fn stop(mut self, stop: Stop) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn response_format(mut self, response_format: ResponseFormat) -> Self {
        self.response_format = Some(response_format);
        self
    }

    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    pub fn stream_options(mut self, stream_options: StreamOptions) -> Self {
        self.stream_options = Some(stream_options);
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Reasoning effort for reasoning models: `"minimal"`, `"low"`,
    /// `"medium"`, or `"high"`.
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    pub fn parallel_tool_calls(mut self, parallel: bool) -> Self {
        self.parallel_tool_calls = Some(parallel);
        self
    }

    pub fn metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn store(mut self, store: bool) -> Self {
        self.store = Some(store);
        self
    }

    /// Set an arbitrary extra parameter on the request body (escape hatch
    /// for provider-specific or newly-added params).
    pub fn param(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

/// A tool call made by the model.
///
/// Exactly one of `function` / `custom` is set, matching `call_type`
/// (`"function"` / `"custom"`). Also serialized when replaying assistant
/// messages, so both are omitted when `None` to keep the wire shape of a
/// function call free of a `custom` key (and vice versa).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<CustomToolCall>,
}

/// The name and raw input of a custom (free-form) tool call, mirroring
/// `chat_completion_message_custom_tool_call.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolCall {
    pub name: String,
    /// Free-form input string generated by the model.
    pub input: String,
}

/// The function name and JSON-encoded arguments of a [`ToolCall`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    /// JSON-encoded arguments string (may be invalid JSON — validate before use).
    pub arguments: String,
}

/// Log-probability information for one generated token, mirroring
/// `chat_completion_token_logprob.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TokenLogprob {
    pub token: String,
    pub logprob: f64,
    /// UTF-8 byte values of the token, when it isn't valid UTF-8 on its own.
    #[serde(default)]
    pub bytes: Option<Vec<u8>>,
    #[serde(default)]
    pub top_logprobs: Vec<TopLogprob>,
}

/// One of the most likely alternative tokens at a position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TopLogprob {
    pub token: String,
    pub logprob: f64,
    #[serde(default)]
    pub bytes: Option<Vec<u8>>,
}

/// Log-probability information for a chat choice
/// (`choice_logprobs` in the Python SDK).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatLogprobs {
    #[serde(default)]
    pub content: Option<Vec<TokenLogprob>>,
    #[serde(default)]
    pub refusal: Option<Vec<TokenLogprob>>,
}

/// The assistant message inside a completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatCompletionMessage {
    pub role: Role,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// One generated completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Choice {
    pub index: u32,
    pub message: ChatCompletionMessage,
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<ChatLogprobs>,
}

/// Response from `POST /chat/completions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatCompletion {
    pub id: String,
    pub choices: Vec<Choice>,
    pub created: i64,
    pub model: String,
    #[serde(default)]
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    #[serde(default)]
    pub usage: Option<Usage>,
    /// Key-value pairs attached at creation (`store: true`) or via `update`.
    /// Read leniently: non-string values are stringified and `null` entries
    /// dropped, so an odd provider value never fails the whole completion.
    #[serde(
        default,
        deserialize_with = "lenient_string_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata: Option<HashMap<String, String>>,
    /// Input/output moderation results, when moderation was requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderation: Option<serde_json::Value>,
}

impl ChatCompletion {
    /// Content of the first choice, if any — the common case.
    pub fn content(&self) -> Option<&str> {
        self.choices.first()?.message.content.as_deref()
    }
}

impl HasId for ChatCompletion {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// A message of a stored chat completion, from
/// `GET /chat/completions/{id}/messages`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatCompletionStoreMessage {
    pub id: String,
    /// The `text` and `image_url` parts when the message was sent as a parts
    /// array; null otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_parts: Option<Vec<serde_json::Value>>,
    #[serde(flatten)]
    pub message: ChatCompletionMessage,
}

impl HasId for ChatCompletionStoreMessage {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// Result of `DELETE /chat/completions/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatCompletionDeleted {
    pub id: String,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub object: String,
}

/// Query parameters for `GET /chat/completions` (stored completions).
#[derive(Debug, Clone, Default)]
pub struct ChatCompletionListParams {
    pub after: Option<String>,
    pub limit: Option<u32>,
    /// Only completions whose metadata contains all of these pairs; sent as
    /// `metadata[key]=value`.
    pub metadata: Option<HashMap<String, String>>,
    pub model: Option<String>,
    pub order: Option<String>,
}

impl ChatCompletionListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Sort order by timestamp: `"asc"` (default) or `"desc"`.
    pub fn order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        let mut query = cursor_query(
            self.after.as_deref(),
            None,
            self.limit,
            self.order.as_deref(),
        );
        if let Some(model) = &self.model {
            query.push(("model".to_string(), model.clone()));
        }
        if let Some(metadata) = &self.metadata {
            query.extend(bracket_query("metadata", metadata));
        }
        query
    }
}

/// Query parameters for `GET /chat/completions/{id}/messages`.
#[derive(Debug, Clone, Default)]
pub struct ChatCompletionMessagesListParams {
    pub after: Option<String>,
    pub limit: Option<u32>,
    pub order: Option<String>,
}

impl ChatCompletionMessagesListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sort order by timestamp: `"asc"` (default) or `"desc"`.
    pub fn order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        cursor_query(
            self.after.as_deref(),
            None,
            self.limit,
            self.order.as_deref(),
        )
    }
}

/// Incremental tool call fragment in a streamed delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChoiceDeltaToolCall {
    pub index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<ChoiceDeltaFunction>,
}

/// Incremental function-call fragment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChoiceDeltaFunction {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

/// The incremental message content in a streamed chunk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChoiceDelta {
    #[serde(default)]
    pub role: Option<Role>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChoiceDeltaToolCall>>,
    /// Partial audio output; the final audio update carries only
    /// `expires_at`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChunkDeltaAudio>,
}

/// A partial audio response inside a streamed delta.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChunkDeltaAudio {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Base64-encoded audio bytes in the requested format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Unix timestamp (seconds) after which the audio is no longer
    /// available for multi-turn use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
}

/// One choice inside a streamed chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChunkChoice {
    pub index: u32,
    #[serde(default)]
    pub delta: ChoiceDelta,
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<ChatLogprobs>,
}

/// A streamed chunk from `POST /chat/completions` with `stream: true`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChatCompletionChunk {
    pub id: String,
    #[serde(default)]
    pub choices: Vec<ChunkChoice>,
    pub created: i64,
    pub model: String,
    #[serde(default)]
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Present on the final chunk when `stream_options.include_usage` is set.
    #[serde(default)]
    pub usage: Option<Usage>,
}

impl ChatCompletionChunk {
    /// Content delta of the first choice, if any.
    pub fn content(&self) -> Option<&str> {
        self.choices.first()?.delta.content.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multimodal_content_parts_serialize() {
        let message = Message::user(vec![
            ContentPart::text("What is in this image?"),
            ContentPart::image_url_with_detail("https://example.com/cat.png", "low"),
            ContentPart::input_audio("aGVsbG8=", "wav"),
        ]);
        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            serde_json::json!({
                "role": "user",
                "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {"type": "image_url", "image_url": {"url": "https://example.com/cat.png", "detail": "low"}},
                    {"type": "input_audio", "input_audio": {"data": "aGVsbG8=", "format": "wav"}}
                ]
            })
        );
    }

    #[test]
    fn string_content_still_serializes_as_plain_string() {
        let message = Message::user("hi");
        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            serde_json::json!({"role": "user", "content": "hi"})
        );
    }

    #[test]
    fn prompt_cache_options_emit_only_chat_fields() {
        let request = ChatCompletionRequest::new("gpt-4o", vec![Message::user("hi")])
            .prompt_cache_options(ChatPromptCacheOptions {
                mode: Some("explicit".into()),
                ttl: Some("30m".into()),
            });
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json["prompt_cache_options"],
            serde_json::json!({"mode": "explicit", "ttl": "30m"})
        );
        let text = json.to_string();
        assert!(!text.contains("prewarm"));
        assert!(!text.contains("comparison_response_id"));

        let empty = ChatCompletionRequest::new("gpt-4o", vec![Message::user("hi")]);
        assert!(serde_json::to_value(&empty)
            .unwrap()
            .get("prompt_cache_options")
            .is_none());
    }

    #[test]
    fn chunk_delta_audio_parses() {
        let chunk: ChatCompletionChunk = serde_json::from_value(serde_json::json!({
            "id": "c1", "object": "chat.completion.chunk", "created": 1, "model": "gpt-4o-audio",
            "choices": [{"index": 0, "delta": {"audio": {
                "id": "audio_1", "data": "AAEC", "transcript": "hi", "expires_at": 1700000000
            }}}]
        }))
        .unwrap();
        let audio = chunk.choices[0].delta.audio.as_ref().unwrap();
        assert_eq!(audio.id.as_deref(), Some("audio_1"));
        assert_eq!(audio.data.as_deref(), Some("AAEC"));
        assert_eq!(audio.transcript.as_deref(), Some("hi"));
        assert_eq!(audio.expires_at, Some(1_700_000_000));
    }

    #[test]
    fn request_skips_none_fields() {
        let request = ChatCompletionRequest::new("gpt-4o", vec![Message::user("hi")]);
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": "hi"}],
            })
        );
    }

    #[test]
    fn deserializes_real_openai_response() {
        let body = r#"{
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "created": 1728933352,
            "model": "gpt-4o-2024-08-06",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello there!",
                    "refusal": null
                },
                "logprobs": null,
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 19,
                "completion_tokens": 10,
                "total_tokens": 29,
                "completion_tokens_details": {"reasoning_tokens": 0}
            },
            "system_fingerprint": "fp_6b68a8204b"
        }"#;
        let completion: ChatCompletion = serde_json::from_str(body).unwrap();
        assert_eq!(completion.content(), Some("Hello there!"));
        assert_eq!(
            completion.choices[0].finish_reason,
            Some(FinishReason::Stop)
        );
        assert_eq!(completion.usage.as_ref().unwrap().total_tokens, 29);
    }

    #[test]
    fn deserializes_tool_call_response() {
        let body = r#"{
            "id": "chatcmpl-1",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "get_weather", "arguments": "{\"city\":\"Hanoi\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }"#;
        let completion: ChatCompletion = serde_json::from_str(body).unwrap();
        let calls = completion.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(calls[0].function.as_ref().unwrap().name, "get_weather");
        assert_eq!(
            completion.choices[0].finish_reason,
            Some(FinishReason::ToolCalls)
        );
    }

    #[test]
    fn deserializes_stream_chunk() {
        let body = r#"{
            "id": "chatcmpl-1",
            "object": "chat.completion.chunk",
            "created": 1,
            "model": "gpt-4o",
            "choices": [{"index": 0, "delta": {"content": "Hel"}, "finish_reason": null}]
        }"#;
        let chunk: ChatCompletionChunk = serde_json::from_str(body).unwrap();
        assert_eq!(chunk.content(), Some("Hel"));
    }

    #[test]
    fn custom_tool_call_parses() {
        let body = r#"{
            "id": "chatcmpl-1",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-5",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "custom",
                        "custom": {"name": "run_sql", "input": "SELECT 1"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }"#;
        let completion: ChatCompletion = serde_json::from_str(body).unwrap();
        let calls = completion.choices[0].message.tool_calls.as_ref().unwrap();
        assert!(calls[0].function.is_none());
        let custom = calls[0].custom.as_ref().unwrap();
        assert_eq!(custom.name, "run_sql");
        assert_eq!(custom.input, "SELECT 1");
    }

    #[test]
    fn replayed_tool_calls_omit_the_unused_variant_key() {
        let parsed: Vec<ToolCall> = serde_json::from_value(serde_json::json!([
            {
                "id": "call_1", "type": "function",
                "function": {"name": "get_weather", "arguments": "{}"}
            },
            {
                "id": "call_2", "type": "custom",
                "custom": {"name": "run_sql", "input": "SELECT 1"}
            }
        ]))
        .unwrap();
        let message = Message::assistant_tool_calls(parsed);
        let wire = serde_json::to_value(&message).unwrap();
        let calls = wire["tool_calls"].as_array().unwrap();

        assert!(calls[0].get("custom").is_none());
        assert_eq!(calls[0]["function"]["name"], "get_weather");
        assert_eq!(
            calls[0],
            serde_json::json!({
                "id": "call_1", "type": "function",
                "function": {"name": "get_weather", "arguments": "{}"}
            })
        );
        assert!(calls[1].get("function").is_none());
        assert_eq!(calls[1]["custom"]["input"], "SELECT 1");
    }
}
