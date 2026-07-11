//! Client configuration and builder, mirroring `_client.py` env-var
//! fallbacks and `_constants.py` defaults.

use std::time::Duration;

use crate::azure::AzureAuth;
use crate::client::Client;
use crate::error::OpenAIError;

pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
/// Default request timeout (600s), from `_constants.py::DEFAULT_TIMEOUT`.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);
/// Default connect timeout (5s), from `_constants.py::DEFAULT_TIMEOUT`.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Default retry count, from `_constants.py::DEFAULT_MAX_RETRIES`.
pub const DEFAULT_MAX_RETRIES: u32 = 2;

/// Azure-specific settings resolved at build time.
#[derive(Clone)]
pub(crate) struct AzureSettings {
    pub auth: AzureAuth,
    /// Pinned deployment. Deployments endpoints are routed to
    /// `/deployments/{deployment}{path}`; when unset, the deployment is
    /// derived from the request body's `model`. Non-deployment endpoints
    /// (e.g. `/models`, `/files`) are never given a deployment segment,
    /// mirroring `lib/azure.py::_prepare_url`.
    pub deployment: Option<String>,
}

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
    pub(crate) default_query: Vec<(String, String)>,
    pub(crate) azure: Option<AzureSettings>,
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
#[derive(Default, Clone)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    organization: Option<String>,
    project: Option<String>,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    max_retries: Option<u32>,
    default_headers: Vec<(String, String)>,
    azure_endpoint: Option<String>,
    azure_api_version: Option<String>,
    azure_deployment: Option<String>,
    azure_ad_token: Option<String>,
}

impl std::fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("base_url", &self.base_url)
            .field("organization", &self.organization)
            .field("project", &self.project)
            .field("timeout", &self.timeout)
            .field("connect_timeout", &self.connect_timeout)
            .field("max_retries", &self.max_retries)
            .field("azure_endpoint", &self.azure_endpoint)
            .field("azure_api_version", &self.azure_api_version)
            .field("azure_deployment", &self.azure_deployment)
            .field(
                "azure_ad_token",
                &self.azure_ad_token.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
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

    /// Target an Azure OpenAI resource, mirroring the Python `AzureOpenAI`
    /// client: requests go to `{endpoint}/openai[...]` with an `api-version`
    /// query parameter, authenticated via `api-key` header (or a bearer
    /// token set with [`ClientBuilder::azure_ad_token`]).
    ///
    /// The API key falls back to the `AZURE_OPENAI_API_KEY` environment
    /// variable; `api_version` may be empty to use `OPENAI_API_VERSION`.
    pub fn azure(mut self, endpoint: impl Into<String>, api_version: impl Into<String>) -> Self {
        self.azure_endpoint = Some(endpoint.into());
        let api_version = api_version.into();
        if !api_version.is_empty() {
            self.azure_api_version = Some(api_version);
        }
        self
    }

    /// Pin all requests to a specific Azure deployment
    /// (`{endpoint}/openai/deployments/{deployment}`). Without this, the
    /// deployment is derived per request from the body's `model` field.
    pub fn azure_deployment(mut self, deployment: impl Into<String>) -> Self {
        self.azure_deployment = Some(deployment.into());
        self
    }

    /// Authenticate to Azure with an Entra ID (Azure AD) bearer token
    /// instead of an API key. Falls back to `AZURE_OPENAI_AD_TOKEN`.
    pub fn azure_ad_token(mut self, token: impl Into<String>) -> Self {
        self.azure_ad_token = Some(token.into());
        self
    }

    /// Resolve environment fallbacks and construct the [`Client`].
    pub fn build(self) -> Result<Client, OpenAIError> {
        let is_azure = self.azure_endpoint.is_some();
        let api_key = self
            .api_key
            .or_else(|| {
                if is_azure {
                    std::env::var("AZURE_OPENAI_API_KEY").ok()
                } else {
                    None
                }
            })
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .filter(|k| !k.trim().is_empty());

        let azure_ad_token = self
            .azure_ad_token
            .or_else(|| {
                if is_azure {
                    std::env::var("AZURE_OPENAI_AD_TOKEN").ok()
                } else {
                    None
                }
            })
            .filter(|t| !t.trim().is_empty());

        let (api_key, base_url, default_query, azure) = if let Some(endpoint) = self.azure_endpoint
        {
            let api_version = self
                .azure_api_version
                .or_else(|| std::env::var("OPENAI_API_VERSION").ok())
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| {
                    OpenAIError::Config(
                        "Azure requires an api_version: pass it to `.azure()` or set OPENAI_API_VERSION"
                            .into(),
                    )
                })?;
            let auth = match (&azure_ad_token, &api_key) {
                (Some(token), _) => AzureAuth::BearerToken(token.clone()),
                (None, Some(key)) => AzureAuth::ApiKey(key.clone()),
                (None, None) => {
                    return Err(OpenAIError::Config(
                        "missing Azure credentials: pass `api_key`/`azure_ad_token` or set AZURE_OPENAI_API_KEY / AZURE_OPENAI_AD_TOKEN"
                            .into(),
                    ))
                }
            };
            // Base URL is always `{endpoint}/openai`; any deployment segment
            // is added per request so non-deployment endpoints stay correct.
            let base_url = crate::azure::azure_base_url(&endpoint, None);
            (
                api_key.unwrap_or_default(),
                base_url,
                vec![("api-version".to_string(), api_version)],
                Some(AzureSettings {
                    auth,
                    deployment: self.azure_deployment,
                }),
            )
        } else {
            let api_key = api_key.ok_or_else(|| {
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
            (api_key, base_url, Vec::new(), None)
        };

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
            default_query,
            azure,
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
