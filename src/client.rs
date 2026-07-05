//! The [`Client`] type: a cheaply-cloneable handle around a shared
//! `reqwest::Client` and resolved [`Config`].

use std::sync::Arc;

use crate::config::{ClientBuilder, Config};
use crate::error::OpenAIError;
use crate::resources::assistants::{Assistants, Threads};
use crate::resources::audio::Audio;
use crate::resources::batches::Batches;
use crate::resources::chat::Chat;
use crate::resources::completions::Completions;
use crate::resources::embeddings::Embeddings;
use crate::resources::files::Files;
use crate::resources::fine_tuning::FineTuning;
use crate::resources::images::Images;
use crate::resources::models::Models;
use crate::resources::moderations::Moderations;
use crate::resources::responses::Responses;
use crate::resources::uploads::Uploads;
use crate::resources::vector_stores::VectorStores;

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

    /// The batches resource.
    pub fn batches(&self) -> Batches {
        Batches::new(self.clone())
    }

    /// The resumable uploads resource.
    pub fn uploads(&self) -> Uploads {
        Uploads::new(self.clone())
    }

    /// Fine-tuning endpoints: `client.fine_tuning().jobs()`.
    pub fn fine_tuning(&self) -> FineTuning {
        FineTuning::new(self.clone())
    }

    /// The vector stores resource.
    pub fn vector_stores(&self) -> VectorStores {
        VectorStores::new(self.clone())
    }

    /// The assistants resource (beta v2): `client.assistants()`.
    pub fn assistants(&self) -> Assistants {
        Assistants::new(self.clone())
    }

    /// The threads resource (beta v2): `client.threads()`, with nested
    /// `.messages()` and `.runs()`.
    pub fn threads(&self) -> Threads {
        Threads::new(self.clone())
    }

    /// The Responses API resource: `client.responses()`.
    pub fn responses(&self) -> Responses {
        Responses::new(self.clone())
    }

    /// Open a realtime WebSocket session for `model`, using this client's
    /// API key, base URL, and organization/project settings.
    ///
    /// Azure realtime (deployment paths, `api-version` query, `api-key`
    /// header over WebSocket) is not supported; Azure-configured clients
    /// return an error.
    pub async fn connect_realtime(
        &self,
        model: &str,
    ) -> Result<crate::realtime::RealtimeSession, crate::realtime::RealtimeError> {
        let config = self.config();
        if config.azure.is_some() {
            return Err(crate::realtime::RealtimeError::Connect(
                "Azure realtime is not supported; use realtime::connect with explicit options"
                    .into(),
            ));
        }
        crate::realtime::connect(crate::realtime::RealtimeConnectOptions {
            api_key: config.api_key.clone(),
            base_url: config.base_url.clone(),
            model: model.to_string(),
            organization: config.organization.clone(),
            project: config.project.clone(),
            extra_headers: Vec::new(),
        })
        .await
    }
}
