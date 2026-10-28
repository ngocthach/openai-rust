//! Error types mirroring the `openai-python` exception hierarchy
//! (`_exceptions.py`): status-code specific API errors, connection/timeout
//! errors, and configuration errors.

use serde::{Deserialize, Deserializer};

/// Structured error detail returned by the API inside `{"error": {...}}`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ApiErrorDetail {
    /// Human-readable error description.
    pub message: Option<String>,
    /// Error type, e.g. `invalid_request_error`.
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    /// The request parameter that caused the error, if any.
    pub param: Option<String>,
    /// Machine-readable error code. Some providers send numbers, so this
    /// accepts both strings and numbers.
    #[serde(default, deserialize_with = "lenient_string")]
    pub code: Option<String>,
}

fn lenient_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(s),
        Some(other) => Some(other.to_string()),
    })
}

/// Classification of an API error by HTTP status code, mirroring the
/// `APIStatusError` subclasses in `_exceptions.py`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApiErrorKind {
    /// 400
    BadRequest,
    /// 401
    Authentication,
    /// 403
    PermissionDenied,
    /// 404
    NotFound,
    /// 409
    Conflict,
    /// 422
    UnprocessableEntity,
    /// 429
    RateLimit,
    /// 5xx
    InternalServer,
    /// Any other non-success status.
    Other,
}

impl ApiErrorKind {
    pub fn from_status(status: u16) -> Self {
        match status {
            400 => Self::BadRequest,
            401 => Self::Authentication,
            403 => Self::PermissionDenied,
            404 => Self::NotFound,
            409 => Self::Conflict,
            422 => Self::UnprocessableEntity,
            429 => Self::RateLimit,
            s if s >= 500 => Self::InternalServer,
            _ => Self::Other,
        }
    }
}

/// An error response (4xx/5xx) from the API.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ApiError {
    /// HTTP status code.
    pub status: u16,
    /// Status-code classification.
    pub kind: ApiErrorKind,
    /// Full error message: `Error code: {status}`, followed by ` - {message}`
    /// when the JSON body carries a message, or by a truncated snippet of a
    /// non-JSON body.
    pub message: String,
    /// Parsed `{"error": {...}}` body, when present.
    pub detail: Option<ApiErrorDetail>,
    /// Value of the `x-request-id` response header.
    pub request_id: Option<String>,
    /// Raw response body, trimmed and capped at 64 KiB; `None` when empty.
    pub body: Option<String>,
}

/// Longest body kept in [`ApiError::body`], bounding memory and log exposure.
const MAX_BODY_BYTES: usize = 64 * 1024;
/// Longest non-JSON body snippet (in chars) embedded in the error message.
const MAX_SNIPPET_CHARS: usize = 200;

/// Cut `s` to at most `max_bytes` without splitting a character.
fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Trimmed `s` limited to `MAX_SNIPPET_CHARS` chars, ending in `…` when cut.
fn snippet(s: &str) -> String {
    match s.char_indices().nth(MAX_SNIPPET_CHARS) {
        Some((end, _)) => format!("{}…", &s[..end]),
        None => s.to_string(),
    }
}

impl ApiError {
    /// Whether this status code is retryable (408/409/429/5xx), mirroring
    /// `_base_client.py::_should_retry`.
    pub fn is_retryable(&self) -> bool {
        matches!(self.status, 408 | 409 | 429) || self.status >= 500
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)?;
        if let Some(id) = &self.request_id {
            write!(f, " (request_id: {id})")?;
        }
        Ok(())
    }
}

/// All errors produced by this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenAIError {
    /// Client construction / configuration problem (e.g. missing API key).
    #[error("configuration error: {0}")]
    Config(String),
    /// Failed to reach the server.
    #[error("connection error: {0}")]
    Connection(String),
    /// A redirect was refused (cross-origin, or too many hops). Never retried.
    #[error("redirect refused: {0}")]
    Redirect(String),
    /// The request timed out.
    #[error("request timed out")]
    Timeout,
    /// The API returned a 4xx/5xx response.
    /// (Boxed to keep `Result` sizes small.)
    #[error("{0}")]
    Api(Box<ApiError>),
    /// An error occurred while consuming a streaming response.
    #[error("stream error: {0}")]
    Stream(String),
    /// Failed to (de)serialize JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Any other HTTP-level failure.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl OpenAIError {
    /// Build an [`OpenAIError::Api`] from an error response, mirroring
    /// `_base_client.py::_make_status_error_from_response`: the body is parsed
    /// as JSON and the nested `error` object extracted when present.
    ///
    /// The message is `Error code: {status}`, plus ` - {message}` when a JSON
    /// body has a non-empty `message` (in `error` or at the top level). Unlike
    /// the Python SDK, a non-JSON body is shown as a truncated snippet so
    /// gateway errors from compatible providers stay readable.
    pub(crate) fn from_response(status: u16, request_id: Option<String>, body: &str) -> Self {
        let body = body.trim();
        let json = serde_json::from_str::<serde_json::Value>(body).ok();
        let error_value = json.as_ref().map(|v| v.get("error").unwrap_or(v).clone());
        let detail = error_value
            .and_then(|e| serde_json::from_value::<ApiErrorDetail>(e).ok())
            .filter(|d| {
                d.message.is_some()
                    || d.error_type.is_some()
                    || d.param.is_some()
                    || d.code.is_some()
            });

        let message = if let Some(json) = &json {
            let msg = json
                .get("error")
                .unwrap_or(json)
                .get("message")
                .and_then(|m| m.as_str())
                .filter(|m| !m.is_empty());
            match msg {
                Some(msg) => format!("Error code: {status} - {msg}"),
                None => format!("Error code: {status}"),
            }
        } else if body.is_empty() {
            format!("Error code: {status}")
        } else {
            format!("Error code: {status} - {}", snippet(body))
        };

        let body =
            (!body.is_empty()).then(|| truncate_to_char_boundary(body, MAX_BODY_BYTES).to_string());

        Self::Api(Box::new(ApiError {
            status,
            kind: ApiErrorKind::from_status(status),
            message,
            detail,
            request_id,
            body,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_statuses_to_kinds() {
        assert_eq!(ApiErrorKind::from_status(400), ApiErrorKind::BadRequest);
        assert_eq!(ApiErrorKind::from_status(401), ApiErrorKind::Authentication);
        assert_eq!(
            ApiErrorKind::from_status(403),
            ApiErrorKind::PermissionDenied
        );
        assert_eq!(ApiErrorKind::from_status(404), ApiErrorKind::NotFound);
        assert_eq!(ApiErrorKind::from_status(409), ApiErrorKind::Conflict);
        assert_eq!(
            ApiErrorKind::from_status(422),
            ApiErrorKind::UnprocessableEntity
        );
        assert_eq!(ApiErrorKind::from_status(429), ApiErrorKind::RateLimit);
        assert_eq!(ApiErrorKind::from_status(500), ApiErrorKind::InternalServer);
        assert_eq!(ApiErrorKind::from_status(503), ApiErrorKind::InternalServer);
        assert_eq!(ApiErrorKind::from_status(418), ApiErrorKind::Other);
    }

    #[test]
    fn parses_error_body() {
        let body = r#"{"error": {"message": "Invalid API key", "type": "invalid_request_error", "param": null, "code": "invalid_api_key"}}"#;
        let err = OpenAIError::from_response(401, Some("req_123".into()), body);
        let OpenAIError::Api(api) = err else {
            panic!("expected Api error")
        };
        assert_eq!(api.status, 401);
        assert_eq!(api.kind, ApiErrorKind::Authentication);
        assert_eq!(api.request_id.as_deref(), Some("req_123"));
        assert!(!api.is_retryable());
        let detail = api.detail.expect("detail parsed");
        assert_eq!(detail.message.as_deref(), Some("Invalid API key"));
        assert_eq!(detail.code.as_deref(), Some("invalid_api_key"));
    }

    fn api_error(status: u16, body: &str) -> ApiError {
        let OpenAIError::Api(api) = OpenAIError::from_response(status, None, body) else {
            panic!("expected Api error")
        };
        *api
    }

    #[test]
    fn json_body_with_message_appends_message_only() {
        let api = api_error(
            401,
            r#"{"error": {"message": "Invalid API key", "code": "x"}}"#,
        );
        assert_eq!(api.message, "Error code: 401 - Invalid API key");
        // Top-level `message` is honored too.
        assert_eq!(
            api_error(400, r#"{"message": "bad"}"#).message,
            "Error code: 400 - bad"
        );
    }

    #[test]
    fn json_body_without_message_has_bare_prefix() {
        assert_eq!(
            api_error(400, r#"{"detail": "x"}"#).message,
            "Error code: 400"
        );
        assert_eq!(
            api_error(500, r#"{"error": {"type": "server_error"}}"#).message,
            "Error code: 500"
        );
        assert_eq!(
            api_error(500, r#"{"error": {"message": ""}}"#).message,
            "Error code: 500"
        );
        assert_eq!(
            api_error(500, r#"{"error": "oops"}"#).message,
            "Error code: 500"
        );
        // Valid JSON that is not an object.
        assert_eq!(api_error(500, "[1, 2]").message, "Error code: 500");
    }

    #[test]
    fn empty_body_has_bare_prefix_and_no_body() {
        let api = api_error(502, "  \n");
        assert_eq!(api.message, "Error code: 502");
        assert!(api.body.is_none());
    }

    #[test]
    fn html_gateway_error_shows_snippet_and_keeps_body() {
        let html = "<html><body><h1>502 Bad Gateway</h1></body></html>";
        let api = api_error(502, &format!("\n{html}\n"));
        assert_eq!(api.message, format!("Error code: 502 - {html}"));
        assert_eq!(api.body.as_deref(), Some(html));
        assert!(api.detail.is_none());
    }

    #[test]
    fn long_text_body_is_truncated_in_message_only() {
        let text = "a".repeat(500);
        let api = api_error(502, &text);
        assert_eq!(
            api.message,
            format!("Error code: 502 - {}…", "a".repeat(200))
        );
        assert_eq!(api.body.as_deref(), Some(text.as_str()));

        // Exactly at the limit: no ellipsis.
        let exact = "b".repeat(200);
        assert_eq!(
            api_error(502, &exact).message,
            format!("Error code: 502 - {exact}")
        );
    }

    #[test]
    fn snippet_truncation_respects_multibyte_chars() {
        let text = "é".repeat(300);
        let api = api_error(502, &text);
        assert_eq!(
            api.message,
            format!("Error code: 502 - {}…", "é".repeat(200))
        );
    }

    #[test]
    fn stored_body_is_capped_on_a_char_boundary() {
        // 3-byte chars: 64 KiB is not a multiple of 3, so the cut must back up.
        let text = "€".repeat(40_000);
        let api = api_error(502, &text);
        let body = api.body.expect("body kept");
        assert!(body.len() <= MAX_BODY_BYTES);
        assert_eq!(body.len(), MAX_BODY_BYTES / 3 * 3);
        assert!(body.chars().all(|c| c == '€'));

        let ascii = "x".repeat(MAX_BODY_BYTES + 10);
        assert_eq!(api_error(500, &ascii).body.unwrap().len(), MAX_BODY_BYTES);
    }

    #[test]
    fn handles_non_json_body_and_numeric_code() {
        let err = OpenAIError::from_response(502, None, "Bad Gateway");
        let OpenAIError::Api(api) = err else { panic!() };
        assert!(api.detail.is_none());
        assert!(api.is_retryable());
        assert_eq!(api.message, "Error code: 502 - Bad Gateway");

        let err = OpenAIError::from_response(502, None, "");
        let OpenAIError::Api(api) = err else { panic!() };
        assert_eq!(api.message, "Error code: 502");

        let err = OpenAIError::from_response(
            429,
            None,
            r#"{"error": {"message": "slow down", "code": 42}}"#,
        );
        let OpenAIError::Api(api) = err else { panic!() };
        assert_eq!(api.detail.unwrap().code.as_deref(), Some("42"));
    }
}
