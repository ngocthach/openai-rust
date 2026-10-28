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

/// OpenAI data-residency region, selecting a regional API endpoint (see
/// [`ClientBuilder::data_residency`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataResidency {
    /// The default global endpoint, `https://api.openai.com/v1`.
    Global,
    /// United States, `https://us.api.openai.com/v1`.
    Us,
    /// European Union, `https://eu.api.openai.com/v1`.
    Eu,
    /// United Arab Emirates, `https://ae.api.openai.com/v1`.
    Ae,
}

impl DataResidency {
    /// Mirrors `_data_residency.py::_DATA_RESIDENCY_BASE_URLS`.
    pub(crate) fn base_url(self) -> &'static str {
        match self {
            Self::Global => DEFAULT_BASE_URL,
            Self::Us => "https://us.api.openai.com/v1",
            Self::Eu => "https://eu.api.openai.com/v1",
            Self::Ae => "https://ae.api.openai.com/v1",
        }
    }
}

/// Pick the Azure credential, mirroring `lib/azure.py::_resolve_azure_auth`.
///
/// `explicit_*` are the raw caller-supplied values; an explicit blank counts
/// as explicit (and is rejected) rather than silently falling through to the
/// environment and authenticating as a different principal. The environment is
/// consulted only when nothing explicit was given, AD token first. There is
/// deliberately no `OPENAI_API_KEY` fallback: that key belongs to a different
/// service and must not be sent to an Azure endpoint.
pub(crate) fn resolve_azure_auth(
    explicit_key: Option<String>,
    explicit_token: Option<String>,
    env_key: Option<String>,
    env_token: Option<String>,
) -> Result<AzureAuth, OpenAIError> {
    let non_blank = |v: Option<String>| v.filter(|v| !v.trim().is_empty());
    match (explicit_key, explicit_token) {
        (Some(_), Some(_)) => Err(OpenAIError::Config(
            "the `api_key` and `azure_ad_token` arguments are mutually exclusive; only one can be passed at a time"
                .into(),
        )),
        (Some(key), None) => match non_blank(Some(key)) {
            Some(key) => Ok(AzureAuth::ApiKey(key)),
            None => Err(OpenAIError::Config("Azure `api_key` is empty".into())),
        },
        (None, Some(token)) => match non_blank(Some(token)) {
            Some(token) => Ok(AzureAuth::BearerToken(token)),
            None => Err(OpenAIError::Config("Azure `azure_ad_token` is empty".into())),
        },
        (None, None) => match (non_blank(env_token), non_blank(env_key)) {
            (Some(token), _) => Ok(AzureAuth::BearerToken(token)),
            (None, Some(key)) => Ok(AzureAuth::ApiKey(key)),
            (None, None) => Err(OpenAIError::Config(
                "missing Azure credentials: pass `api_key`/`azure_ad_token` or set AZURE_OPENAI_API_KEY / AZURE_OPENAI_AD_TOKEN"
                    .into(),
            )),
        },
    }
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
    data_residency: Option<DataResidency>,
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
            .field("data_residency", &self.data_residency)
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
    /// Credentials: an explicit `api_key` or `azure_ad_token` (not both)
    /// wins; only when neither is given do `AZURE_OPENAI_AD_TOKEN` (first) and
    /// `AZURE_OPENAI_API_KEY` apply. `OPENAI_API_KEY` is never used for Azure.
    /// `api_version` may be empty to use `OPENAI_API_VERSION`.
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
    /// instead of an API key. Falls back to `AZURE_OPENAI_AD_TOKEN` only when
    /// no explicit credential is set.
    pub fn azure_ad_token(mut self, token: impl Into<String>) -> Self {
        self.azure_ad_token = Some(token.into());
        self
    }

    /// Send requests to a data-residency endpoint
    /// (`https://{region}.api.openai.com/v1`). Mutually exclusive with
    /// [`ClientBuilder::base_url`] and Azure (checked by `build`); takes
    /// precedence over `OPENAI_BASE_URL`.
    pub fn data_residency(mut self, region: DataResidency) -> Self {
        self.data_residency = Some(region);
        self
    }

    /// Resolve environment fallbacks and construct the [`Client`].
    pub fn build(self) -> Result<Client, OpenAIError> {
        if self.data_residency.is_some()
            && (self.azure_endpoint.is_some() || self.base_url.is_some())
        {
            return Err(OpenAIError::Config(
                "`data_residency` is mutually exclusive with `base_url` and Azure".into(),
            ));
        }

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
            let auth = resolve_azure_auth(
                self.api_key,
                self.azure_ad_token,
                std::env::var("AZURE_OPENAI_API_KEY").ok(),
                std::env::var("AZURE_OPENAI_AD_TOKEN").ok(),
            )?;
            let api_key = match &auth {
                AzureAuth::ApiKey(key) => key.clone(),
                AzureAuth::BearerToken(_) => String::new(),
            };
            // Base URL is always `{endpoint}/openai`; any deployment segment
            // is added per request so non-deployment endpoints stay correct.
            let base_url = crate::azure::azure_base_url(&endpoint, None);
            (
                api_key,
                base_url,
                vec![("api-version".to_string(), api_version)],
                Some(AzureSettings {
                    auth,
                    deployment: self.azure_deployment,
                }),
            )
        } else {
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
            // `data_residency` names the endpoint outright, so it also wins
            // over `OPENAI_BASE_URL`.
            let base_url = match self.data_residency {
                Some(region) => region.base_url().to_string(),
                None => self
                    .base_url
                    .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
                    .filter(|u| !u.trim().is_empty())
                    .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            };
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

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    fn resolve(
        key: Option<&str>,
        token: Option<&str>,
        env_key: Option<&str>,
        env_token: Option<&str>,
    ) -> Result<AzureAuth, OpenAIError> {
        resolve_azure_auth(
            key.and_then(s),
            token.and_then(s),
            env_key.and_then(s),
            env_token.and_then(s),
        )
    }

    #[test]
    fn azure_explicit_key_beats_env_token() {
        let auth = resolve(Some("k"), None, None, Some("env-token")).unwrap();
        assert!(matches!(auth, AzureAuth::ApiKey(k) if k == "k"));
    }

    #[test]
    fn azure_explicit_token_beats_env_key() {
        let auth = resolve(None, Some("t"), Some("env-key"), None).unwrap();
        assert!(matches!(auth, AzureAuth::BearerToken(t) if t == "t"));
    }

    #[test]
    fn azure_both_explicit_is_error() {
        let err = resolve(Some("k"), Some("t"), None, None).unwrap_err();
        assert!(matches!(err, OpenAIError::Config(_)));
    }

    #[test]
    fn azure_explicit_blank_is_error_not_env_fallthrough() {
        assert!(resolve(Some(""), None, Some("env-key"), Some("env-token")).is_err());
        assert!(resolve(None, Some("  "), Some("env-key"), Some("env-token")).is_err());
    }

    #[test]
    fn azure_env_token_beats_env_key() {
        let auth = resolve(None, None, Some("env-key"), Some("env-token")).unwrap();
        assert!(matches!(auth, AzureAuth::BearerToken(t) if t == "env-token"));
    }

    #[test]
    fn azure_env_key_used_when_no_token() {
        let auth = resolve(None, None, Some("env-key"), None).unwrap();
        assert!(matches!(auth, AzureAuth::ApiKey(k) if k == "env-key"));
    }

    #[test]
    fn azure_blank_env_is_unset() {
        assert!(resolve(None, None, Some(" "), Some("")).is_err());
    }

    #[test]
    fn data_residency_urls() {
        let url = |r| {
            ClientBuilder::new()
                .api_key("sk")
                .data_residency(r)
                .build()
                .unwrap()
                .base_url()
                .to_string()
        };
        assert_eq!(url(DataResidency::Global), "https://api.openai.com/v1");
        assert_eq!(url(DataResidency::Us), "https://us.api.openai.com/v1");
        assert_eq!(url(DataResidency::Eu), "https://eu.api.openai.com/v1");
        assert_eq!(url(DataResidency::Ae), "https://ae.api.openai.com/v1");
    }

    #[test]
    fn data_residency_conflicts_are_config_errors() {
        let with_base = ClientBuilder::new()
            .api_key("sk")
            .base_url("https://example.com/v1")
            .data_residency(DataResidency::Eu)
            .build();
        assert!(matches!(with_base, Err(OpenAIError::Config(_))));
        let with_azure = ClientBuilder::new()
            .api_key("sk")
            .azure("https://x.openai.azure.com", "2024-02-01")
            .data_residency(DataResidency::Eu)
            .build();
        assert!(matches!(with_azure, Err(OpenAIError::Config(_))));
    }
}
