//! Async Rust client for OpenAI-compatible LLM provider APIs.
//!
//! A port of the core of the official `openai-python` SDK: chat completions
//! (with SSE streaming), embeddings, models, moderations, images, files,
//! audio, automatic retries with exponential backoff, and typed errors.
//!
//! The `base_url` is configurable, so any OpenAI-compatible provider works.
//!
//! ```no_run
//! use openai_compat::Client;
//!
//! # async fn run() -> Result<(), openai_compat::OpenAIError> {
//! // Reads OPENAI_API_KEY (and optional OPENAI_BASE_URL) from the environment.
//! let client = Client::new()?;
//! # Ok(())
//! # }
//! ```

mod azure;
mod client;
mod config;
mod error;
mod pagination;
pub mod realtime;
mod request;
pub mod resources;
mod retry;
mod streaming;
pub mod types;
pub mod webhooks;

pub use client::Client;
pub use config::{
    ClientBuilder, DataResidency, DEFAULT_BASE_URL, DEFAULT_CONNECT_TIMEOUT, DEFAULT_MAX_RETRIES,
    DEFAULT_TIMEOUT,
};
pub use error::{ApiError, ApiErrorDetail, ApiErrorKind, OpenAIError};
pub use pagination::{HasId, List};
pub use streaming::{EventStream, ServerSentEvent, SseDecoder};

// Convenience re-exports of the most-used types.
pub use types::chat::{
    ChatCompletion, ChatCompletionChunk, ChatCompletionRequest, ChatPromptCacheOptions,
    ContentPart, Message, MessageContent, Stop, StreamOptions,
};
pub use types::common::{FinishReason, ResponseFormat, Role, Tool, ToolChoice, Usage};
pub use types::responses::{
    CreateResponseRequest, FileDetail, Input, InputItem, Reasoning, Response, ResponseStreamEvent,
    ResponsesPromptCacheOptions,
};
pub use webhooks::{WebhookEvent, WebhookHeaders, Webhooks};
