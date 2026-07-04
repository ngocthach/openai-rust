//! Legacy completions types, mirroring
//! `openai-python/src/openai/types/completion*.py`.

use serde::{Deserialize, Serialize};

use super::chat::{Stop, StreamOptions};
use super::common::{FinishReason, Usage};

/// `prompt` accepts a string, array of strings, or (arrays of) token ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Prompt {
    Text(String),
    Texts(Vec<String>),
    Tokens(Vec<i64>),
    TokenArrays(Vec<Vec<i64>>),
}

impl From<&str> for Prompt {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<String> for Prompt {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

/// Request body for the legacy `POST /completions`
/// (`resources/completions.py::create`).
#[derive(Debug, Clone, Serialize)]
pub struct CompletionRequest {
    pub model: String,
    pub prompt: Prompt,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_of: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl CompletionRequest {
    pub fn new(model: impl Into<String>, prompt: impl Into<Prompt>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            best_of: None,
            echo: None,
            frequency_penalty: None,
            logprobs: None,
            max_tokens: None,
            n: None,
            presence_penalty: None,
            seed: None,
            stop: None,
            stream: None,
            stream_options: None,
            suffix: None,
            temperature: None,
            top_p: None,
            user: None,
        }
    }

    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
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

    pub fn stop(mut self, stop: Stop) -> Self {
        self.stop = Some(stop);
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

    pub fn echo(mut self, echo: bool) -> Self {
        self.echo = Some(echo);
        self
    }

    pub fn best_of(mut self, best_of: u32) -> Self {
        self.best_of = Some(best_of);
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
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

    /// Number of top logprobs to include per token (0-5).
    pub fn logprobs(mut self, logprobs: u32) -> Self {
        self.logprobs = Some(logprobs);
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn stream_options(mut self, stream_options: StreamOptions) -> Self {
        self.stream_options = Some(stream_options);
        self
    }
}

/// Log-probability information for a legacy completion choice, mirroring
/// `completion_choice.py::Logprobs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CompletionLogprobs {
    #[serde(default)]
    pub text_offset: Option<Vec<u64>>,
    #[serde(default)]
    pub token_logprobs: Option<Vec<Option<f64>>>,
    #[serde(default)]
    pub tokens: Option<Vec<String>>,
    #[serde(default)]
    pub top_logprobs: Option<Vec<Option<std::collections::HashMap<String, f64>>>>,
}

/// One generated completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CompletionChoice {
    pub text: String,
    pub index: u32,
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<CompletionLogprobs>,
}

/// Response from `POST /completions` (also the streamed chunk shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Completion {
    pub id: String,
    pub choices: Vec<CompletionChoice>,
    pub created: i64,
    pub model: String,
    #[serde(default)]
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

impl Completion {
    /// Text of the first choice, if any.
    pub fn text(&self) -> Option<&str> {
        self.choices.first().map(|c| c.text.as_str())
    }
}
