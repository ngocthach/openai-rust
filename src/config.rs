//! Client configuration and builder, mirroring `_client.py` env-var
//! fallbacks and `_constants.py` defaults.

use std::time::Duration;

use crate::client::Client;
use crate::error::OpenAIError;

pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
/// Default request timeout (600s), from `_constants.py::DEFAULT_TIMEOUT`.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);
/// Default connect timeout (5s), from `_constants.py::DEFAULT_TIMEOUT`.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Default retry count, from `_constants.py::DEFAULT_MAX_RETRIES`.
pub const DEFAULT_MAX_RETRIES: u32 = 2;

/// Resolved client configuration.
#[derive(Clone)]
pub struct Config {
    pub(crate) api_key: String,
    pub(crate) base_url: String,
    pub(crate) organization: Option<String>,
    pub(crate) project: Option<String>,
    pub(crate) timeout: Duration,
    pub(crate) connect_timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) default_headers: Vec<(String, String)>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("api_key", &"[REDACTED]")
            .field("base_url", &self.base_url)
            .field("organization", &self.organization)
            .field("project", &self.project)
            .field("timeout", &self.timeout)
            .field("connect_timeout", &self.connect_timeout)
            .field("max_retries", &self.max_retries)
            .finish()
    }
}

/// Builder for [`Client`]. Unset fields fall back to the same environment
/// variables the Python SDK uses: `OPENAI_API_KEY`, `OPENAI_BASE_URL`,
/// `OPENAI_ORG_ID`, `OPENAI_PROJECT_ID`.
#[derive(Debug, Default, Clone)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    organization: Option<String>,
    project: Option<String>,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    max_retries: Option<u32>,
    default_headers: Vec<(String, String)>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// API key used in the `Authorization: Bearer` header.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Base URL of the API, e.g. `https://api.openai.com/v1` or any
    /// OpenAI-compatible provider endpoint.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Sent as the `OpenAI-Organization` header.
    pub fn organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }

    /// Sent as the `OpenAI-Project` header.
    pub fn project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Read timeout, applied per read operation (mirroring the Python SDK's
    /// httpx behavior) so streaming responses are not cut off by a total
    /// deadline. Defaults to 600 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Connection timeout. Defaults to 5 seconds.
    pub fn connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = Some(connect_timeout);
        self
    }

    /// Maximum number of retries for retryable failures. Defaults to 2.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Add a header sent with every request.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.push((name.into(), value.into()));
        self
    }

    /// Resolve environment fallbacks and construct the [`Client`].
    pub fn build(self) -> Result<Client, OpenAIError> {
        let api_key = self
            .api_key
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .filter(|k| !k.trim().is_empty())
            .ok_or_else(|| {
                OpenAIError::Config(
                    "missing API key: pass `api_key` or set the OPENAI_API_KEY environment variable"
                        .into(),
                )
            })?;

        let base_url = self
            .base_url
            .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let base_url = base_url.trim_end_matches('/').to_string();

        let config = Config {
            api_key,
            base_url,
            organization: self
                .organization
                .or_else(|| std::env::var("OPENAI_ORG_ID").ok()),
            project: self
                .project
                .or_else(|| std::env::var("OPENAI_PROJECT_ID").ok()),
            timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
            connect_timeout: self.connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT),
            max_retries: self.max_retries.unwrap_or(DEFAULT_MAX_RETRIES),
            default_headers: self.default_headers,
        };
        Client::from_config(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_api_key_is_config_error() {
        // Only meaningful when the env var is absent; skip otherwise.
        if std::env::var("OPENAI_API_KEY").is_ok() {
            return;
        }
        let err = ClientBuilder::new().build().unwrap_err();
        assert!(matches!(err, OpenAIError::Config(_)));
    }

    #[test]
    fn base_url_trailing_slash_is_trimmed() {
        let client = ClientBuilder::new()
            .api_key("sk-test")
            .base_url("https://example.com/v1/")
            .build()
            .unwrap();
        assert_eq!(client.base_url(), "https://example.com/v1");
    }

    #[test]
    fn config_debug_redacts_api_key() {
        let client = ClientBuilder::new().api_key("sk-secret").build().unwrap();
        let debug = format!("{:?}", client.config());
        assert!(!debug.contains("sk-secret"));
        assert!(debug.contains("[REDACTED]"));
    }
}
