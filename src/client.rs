//! The [`Client`] type: a cheaply-cloneable handle around a shared
//! `reqwest::Client` and resolved [`Config`].

use std::sync::Arc;

use crate::config::{ClientBuilder, Config};
use crate::error::OpenAIError;
use crate::resources::audio::Audio;
use crate::resources::chat::Chat;
use crate::resources::completions::Completions;
use crate::resources::embeddings::Embeddings;
use crate::resources::files::Files;
use crate::resources::images::Images;
use crate::resources::models::Models;
use crate::resources::moderations::Moderations;

/// Asynchronous client for OpenAI-compatible APIs.
///
/// Construct with [`Client::new`] (environment variables) or
/// [`Client::builder`]. Cloning is cheap (shared connection pool).
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    http: reqwest::Client,
    config: Config,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("config", &self.inner.config)
            .finish()
    }
}

impl Client {
    /// Build a client from environment variables (`OPENAI_API_KEY`,
    /// `OPENAI_BASE_URL`, `OPENAI_ORG_ID`, `OPENAI_PROJECT_ID`).
    pub fn new() -> Result<Self, OpenAIError> {
        Self::builder().build()
    }

    /// Start configuring a client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub(crate) fn from_config(config: Config) -> Result<Self, OpenAIError> {
        // Mirror httpx semantics from the Python SDK: the timeout applies per
        // read operation, not as a whole-request deadline — otherwise
        // long-lived SSE streams would be aborted mid-flight. A total
        // deadline can still be set per request via RequestOptions::timeout.
        let http = reqwest::Client::builder()
            .read_timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .build()
            .map_err(|e| OpenAIError::Config(format!("failed to build HTTP client: {e}")))?;
        Ok(Self {
            inner: Arc::new(Inner { http, config }),
        })
    }

    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }

    pub(crate) fn config(&self) -> &Config {
        &self.inner.config
    }

    /// The resolved base URL (no trailing slash).
    pub fn base_url(&self) -> &str {
        &self.inner.config.base_url
    }

    /// Chat endpoints: `client.chat().completions()`.
    pub fn chat(&self) -> Chat {
        Chat::new(self.clone())
    }

    /// The embeddings resource.
    pub fn embeddings(&self) -> Embeddings {
        Embeddings::new(self.clone())
    }

    /// The models resource.
    pub fn models(&self) -> Models {
        Models::new(self.clone())
    }

    /// The moderations resource.
    pub fn moderations(&self) -> Moderations {
        Moderations::new(self.clone())
    }

    /// The legacy completions resource.
    pub fn completions(&self) -> Completions {
        Completions::new(self.clone())
    }

    /// The images resource.
    pub fn images(&self) -> Images {
        Images::new(self.clone())
    }

    /// The files resource.
    pub fn files(&self) -> Files {
        Files::new(self.clone())
    }

    /// Audio endpoints (speech, transcriptions).
    pub fn audio(&self) -> Audio {
        Audio::new(self.clone())
    }
}
