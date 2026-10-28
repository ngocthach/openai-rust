//! Pure helpers for Azure OpenAI support, mirroring `openai-python`'s
//! `lib/azure.py`.
//!
//! These are stateless functions and small value types with no dependency on
//! [`crate::Config`] or the request pipeline; a later phase wires them into
//! config/request construction (base URL, `api-version` query, auth header, and
//! deployment path rewriting).

use crate::error::OpenAIError;
use crate::request::encode_path_segment;

/// Endpoints that are addressed as `/deployments/{deployment}{path}` on Azure,
/// mirroring `_deployments_endpoints` in `lib/azure.py`.
pub const DEPLOYMENTS_ENDPOINTS: [&str; 8] = [
    "/completions",
    "/chat/completions",
    "/embeddings",
    "/audio/transcriptions",
    "/audio/translations",
    "/audio/speech",
    "/images/generations",
    "/images/edits",
];

/// Azure authentication method.
#[derive(Clone)]
pub enum AzureAuth {
    /// A resource API key, sent as the `api-key` header.
    ApiKey(String),
    /// An Azure AD / Entra ID bearer token, sent as `Authorization: Bearer ...`.
    BearerToken(String),
}

impl std::fmt::Debug for AzureAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey(_) => f.write_str("ApiKey([REDACTED])"),
            Self::BearerToken(_) => f.write_str("BearerToken([REDACTED])"),
        }
    }
}

impl AzureAuth {
    /// The `(header-name, header-value)` pair for this auth method, mirroring
    /// `_auth_headers` in `lib/azure.py`.
    ///
    /// - [`AzureAuth::ApiKey`] → `("api-key", "{key}")`
    /// - [`AzureAuth::BearerToken`] → `("authorization", "Bearer {token}")`
    pub fn header(&self) -> (&'static str, String) {
        match self {
            AzureAuth::ApiKey(key) => ("api-key", key.clone()),
            AzureAuth::BearerToken(token) => ("authorization", format!("Bearer {token}")),
        }
    }
}

/// Build the Azure base URL, mirroring `AzureOpenAI.__init__` base_url logic:
///
/// - with a deployment → `{endpoint}/openai/deployments/{deployment}`
/// - without a deployment → `{endpoint}/openai`
///
/// The trailing `/` of `endpoint` is trimmed first.
pub fn azure_base_url(endpoint: &str, deployment: Option<&str>) -> String {
    let endpoint = endpoint.trim_end_matches('/');
    match deployment {
        Some(dep) => format!("{endpoint}/openai/deployments/{dep}"),
        None => format!("{endpoint}/openai"),
    }
}

/// Rewrite a request path for Azure deployment routing, mirroring
/// `_build_request` in `lib/azure.py`.
///
/// If **no** deployment is configured on the client (`has_deployment == false`),
/// the request path is one of [`DEPLOYMENTS_ENDPOINTS`], and the JSON body
/// carries a `model` (`model_from_body == Some(_)`), the path is rewritten to
/// `/deployments/{model}{path}`. In every other case the path is returned
/// unchanged.
///
/// Divergences from `_build_request`:
/// - Python checks `"/deployments" not in base_url.path`; here that condition is
///   passed explicitly as `has_deployment` (a configured deployment always puts
///   `/deployments/{dep}` in the base URL, so the two are equivalent).
/// - Python only rewrites when `json_data` is a mapping; that is captured by
///   `model_from_body` being `None` whenever there is no JSON object body or no
///   `model` field.
///
/// The model is percent-encoded as one path segment (and `.`/`..` rejected), so
/// a hostile `model` value cannot alter the request path.
pub fn rewrite_path(
    path: &str,
    has_deployment: bool,
    model_from_body: Option<&str>,
) -> Result<String, OpenAIError> {
    if !has_deployment && DEPLOYMENTS_ENDPOINTS.contains(&path) {
        if let Some(model) = model_from_body {
            return Ok(format!(
                "/deployments/{}{path}",
                encode_path_segment(model)?
            ));
        }
    }
    Ok(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_with_deployment() {
        assert_eq!(
            azure_base_url("https://example.openai.azure.com", Some("gpt-4o")),
            "https://example.openai.azure.com/openai/deployments/gpt-4o"
        );
    }

    #[test]
    fn base_url_without_deployment() {
        assert_eq!(
            azure_base_url("https://example.openai.azure.com", None),
            "https://example.openai.azure.com/openai"
        );
    }

    #[test]
    fn base_url_trims_trailing_slash() {
        assert_eq!(
            azure_base_url("https://example.openai.azure.com/", Some("dep")),
            "https://example.openai.azure.com/openai/deployments/dep"
        );
        assert_eq!(
            azure_base_url("https://example.openai.azure.com/", None),
            "https://example.openai.azure.com/openai"
        );
    }

    #[test]
    fn rewrite_path_adds_deployment_for_known_endpoint_with_model() {
        assert_eq!(
            rewrite_path("/chat/completions", false, Some("gpt-4o")).unwrap(),
            "/deployments/gpt-4o/chat/completions"
        );
        assert_eq!(
            rewrite_path("/embeddings", false, Some("text-embedding-3-small")).unwrap(),
            "/deployments/text-embedding-3-small/embeddings"
        );
    }

    #[test]
    fn rewrite_path_unchanged_when_deployment_configured() {
        // A configured deployment already carries /deployments in the base URL.
        assert_eq!(
            rewrite_path("/chat/completions", true, Some("gpt-4o")).unwrap(),
            "/chat/completions"
        );
    }

    #[test]
    fn rewrite_path_unchanged_without_model() {
        assert_eq!(
            rewrite_path("/chat/completions", false, None).unwrap(),
            "/chat/completions"
        );
    }

    #[test]
    fn rewrite_path_unchanged_for_non_deployment_endpoint() {
        // Not in DEPLOYMENTS_ENDPOINTS -> unchanged even with a model.
        assert_eq!(
            rewrite_path("/models", false, Some("gpt-4o")).unwrap(),
            "/models"
        );
        assert_eq!(
            rewrite_path("/files", false, Some("gpt-4o")).unwrap(),
            "/files"
        );
    }

    #[test]
    fn auth_header_api_key() {
        let (name, value) = AzureAuth::ApiKey("secret-key".into()).header();
        assert_eq!(name, "api-key");
        assert_eq!(value, "secret-key");
    }

    #[test]
    fn auth_header_bearer_token() {
        let (name, value) = AzureAuth::BearerToken("ad-token".into()).header();
        assert_eq!(name, "authorization");
        assert_eq!(value, "Bearer ad-token");
    }

    #[test]
    fn deployments_endpoints_match_python() {
        assert_eq!(DEPLOYMENTS_ENDPOINTS.len(), 8);
        assert!(DEPLOYMENTS_ENDPOINTS.contains(&"/audio/speech"));
        assert!(DEPLOYMENTS_ENDPOINTS.contains(&"/images/edits"));
    }

    #[test]
    fn rewrite_path_encodes_model() {
        assert_eq!(
            rewrite_path("/chat/completions", false, Some("org/model")).unwrap(),
            "/deployments/org%2Fmodel/chat/completions"
        );
        assert!(rewrite_path("/chat/completions", false, Some("..")).is_err());
    }
}
