//! Request pipeline with retries, mirroring the send/retry loop in
//! `_base_client.py:1006-1128`.

use std::time::Duration;

use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::retry::{parse_retry_after, retry_delay, should_retry, RetryAfter};

/// A multipart form field. Forms are rebuilt for every retry attempt, so the
/// parts are stored as plain data rather than `reqwest::multipart::Form`.
#[derive(Debug, Clone)]
pub(crate) enum MultipartField {
    Text {
        name: String,
        value: String,
    },
    File {
        name: String,
        filename: String,
        bytes: Vec<u8>,
        content_type: Option<String>,
    },
}

#[derive(Debug, Clone, Default)]
pub(crate) enum Payload {
    #[default]
    None,
    Json(serde_json::Value),
    Multipart(Vec<MultipartField>),
}

/// Per-request options assembled by resource methods.
#[derive(Debug, Clone, Default)]
pub(crate) struct RequestOptions {
    pub query: Vec<(String, String)>,
    pub body: Payload,
    pub extra_headers: Vec<(String, String)>,
    pub timeout: Option<Duration>,
}

impl RequestOptions {
    pub fn json(body: serde_json::Value) -> Self {
        Self {
            body: Payload::Json(body),
            ..Self::default()
        }
    }

    pub fn multipart(fields: Vec<MultipartField>) -> Self {
        Self {
            body: Payload::Multipart(fields),
            ..Self::default()
        }
    }

    pub fn query(query: Vec<(String, String)>) -> Self {
        Self {
            query,
            ..Self::default()
        }
    }
}

/// Percent-encode `value` for use as a single URL path segment, mirroring
/// `_utils/_path.py`: everything outside RFC 3986 `pchar` (unreserved plus
/// `!$&'()*+,;=:@`) is escaped, including `/ ? # %`, so an identifier can never
/// change the shape of the path or smuggle in a query or fragment. `:` stays
/// literal so fine-tuned model ids (`ft:gpt-4o:org::id`) are unchanged.
///
/// `.` and `..` (literal or percent-encoded) are rejected: servers and proxies
/// may normalize them into a different resource. An empty value is rejected
/// too, since `/files/` would address the collection instead of an item.
pub(crate) fn encode_path_segment(value: &str) -> Result<String, OpenAIError> {
    if value.is_empty() {
        return Err(OpenAIError::Config(
            "path parameter must not be empty".into(),
        ));
    }
    let is_dot_segment = {
        let mut rest = value;
        let mut dots = 0;
        while dots < 3 {
            if let Some(r) = rest.strip_prefix('.') {
                rest = r;
            } else if rest.len() >= 3
                && rest.is_char_boundary(3)
                && rest[..3].eq_ignore_ascii_case("%2e")
            {
                rest = &rest[3..];
            } else {
                break;
            }
            dots += 1;
        }
        rest.is_empty() && (1..=2).contains(&dots)
    };
    if is_dot_segment {
        return Err(OpenAIError::Config(format!(
            "path parameter {value:?} is a dot-segment, which is not allowed"
        )));
    }
    let mut out = String::with_capacity(value.len());
    for &b in value.as_bytes() {
        if b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    Ok(out)
}

/// Encode a string map as bracketed query pairs (`prefix[key]=value`), the
/// form used by filter parameters such as chat completion `metadata`. Keys
/// are sorted so the query string is deterministic.
pub(crate) fn bracket_query(
    prefix: &str,
    values: &std::collections::HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut pairs: Vec<_> = values
        .iter()
        .map(|(key, value)| (format!("{prefix}[{key}]"), value.clone()))
        .collect();
    pairs.sort();
    pairs
}

/// `format!` for URL paths: every argument is percent-encoded as one path
/// segment via [`encode_path_segment`] and an invalid argument returns early
/// with `Err(OpenAIError::Config)` from the enclosing function.
macro_rules! api_path {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        format!($fmt, $($crate::request::encode_path_segment(AsRef::<str>::as_ref(&$arg))?),*)
    };
}
pub(crate) use api_path;

fn build_multipart(fields: &[MultipartField]) -> Result<reqwest::multipart::Form, OpenAIError> {
    let mut form = reqwest::multipart::Form::new();
    for field in fields {
        form = match field {
            MultipartField::Text { name, value } => form.text(name.clone(), value.clone()),
            MultipartField::File {
                name,
                filename,
                bytes,
                content_type,
            } => {
                let mut part =
                    reqwest::multipart::Part::bytes(bytes.clone()).file_name(filename.clone());
                if let Some(ct) = content_type {
                    part = part.mime_str(ct).map_err(|e| {
                        OpenAIError::Config(format!("invalid content type {ct:?}: {e}"))
                    })?;
                }
                form.part(name.clone(), part)
            }
        };
    }
    Ok(form)
}

/// Message for a redirect-policy error: the policy's own text (target URL
/// and reason) rather than reqwest's generic wrapper.
fn redirect_message(err: &reqwest::Error) -> String {
    use std::error::Error;
    err.source()
        .map(|s| s.to_string())
        .unwrap_or_else(|| err.to_string())
}

impl Client {
    fn request_headers(&self, extra: &[(String, String)]) -> Result<HeaderMap, OpenAIError> {
        let config = self.config();
        let mut headers = HeaderMap::new();

        if let Some(azure) = &config.azure {
            // Azure auth replaces the default Bearer header entirely:
            // `api-key: <key>` or `Authorization: Bearer <AD token>`.
            let (name, value) = azure.auth.header();
            let mut value = HeaderValue::from_str(&value).map_err(|_| {
                OpenAIError::Config("Azure credential contains invalid header characters".into())
            })?;
            value.set_sensitive(true);
            headers.insert(HeaderName::from_static(name), value);
        } else {
            let mut auth =
                HeaderValue::from_str(&format!("Bearer {}", config.api_key)).map_err(|_| {
                    OpenAIError::Config("API key contains invalid header characters".into())
                })?;
            auth.set_sensitive(true);
            headers.insert(AUTHORIZATION, auth);
        }

        if let Some(org) = &config.organization {
            headers.insert(
                HeaderName::from_static("openai-organization"),
                HeaderValue::from_str(org)
                    .map_err(|_| OpenAIError::Config("invalid organization header value".into()))?,
            );
        }
        if let Some(project) = &config.project {
            headers.insert(
                HeaderName::from_static("openai-project"),
                HeaderValue::from_str(project)
                    .map_err(|_| OpenAIError::Config("invalid project header value".into()))?,
            );
        }
        for (name, value) in config.default_headers.iter().chain(extra.iter()) {
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| OpenAIError::Config(format!("invalid header name {name:?}")))?;
            let value = HeaderValue::from_str(value)
                .map_err(|_| OpenAIError::Config(format!("invalid value for header {name}")))?;
            headers.insert(name, value);
        }
        Ok(headers)
    }

    fn build_request(
        &self,
        method: &Method,
        path: &str,
        options: &RequestOptions,
        attempt: u32,
    ) -> Result<reqwest::RequestBuilder, OpenAIError> {
        let config = self.config();
        let path = if let Some(azure) = &config.azure {
            // Mirror azure.py::_prepare_url: only deployments endpoints get a
            // `/deployments/{...}` segment — from the pinned deployment, or
            // derived from the body's `model` when none is pinned. All other
            // endpoints go straight under `{endpoint}/openai`.
            // The pinned deployment is operator-supplied and used verbatim.
            // Only requests that carry a body (JSON or multipart, e.g. audio
            // transcriptions) are routed, so a body-less GET on a deployments
            // path — such as listing stored chat completions — is left untouched.
            let has_body = !matches!(options.body, Payload::None);
            if let Some(deployment) = azure.deployment.as_deref() {
                if has_body && crate::azure::DEPLOYMENTS_ENDPOINTS.contains(&path) {
                    format!("/deployments/{deployment}{path}")
                } else {
                    path.to_string()
                }
            } else {
                let model = match &options.body {
                    Payload::Json(value) => value.get("model").and_then(|m| m.as_str()),
                    _ => None,
                };
                crate::azure::rewrite_path(path, false, model)?
            }
        } else {
            path.to_string()
        };
        let url = format!("{}{}", self.base_url(), path);
        let mut headers = self.request_headers(&options.extra_headers)?;
        // A caller-supplied retry count wins over the automatic one.
        if !headers.contains_key("x-stainless-retry-count") {
            headers.insert("x-stainless-retry-count", HeaderValue::from(attempt));
        }
        let mut builder = self.http().request(method.clone(), url).headers(headers);
        if !config.default_query.is_empty() {
            builder = builder.query(&config.default_query);
        }
        if !options.query.is_empty() {
            builder = builder.query(&options.query);
        }
        if let Some(timeout) = options.timeout {
            builder = builder.timeout(timeout);
        }
        builder = match &options.body {
            Payload::None => builder,
            Payload::Json(value) => builder.json(value),
            Payload::Multipart(fields) => builder.multipart(build_multipart(fields)?),
        };
        Ok(builder)
    }

    /// Send a request, retrying per `_base_client.py` semantics, and return
    /// the successful raw response (used for streaming and binary bodies).
    pub(crate) async fn execute_raw(
        &self,
        method: Method,
        path: &str,
        options: RequestOptions,
    ) -> Result<reqwest::Response, OpenAIError> {
        let max_retries = self.config().max_retries;

        for attempt in 0..=max_retries {
            let request = self.build_request(&method, path, &options, attempt)?;
            match request.send().await {
                Err(err) => {
                    // Connection-level failure: retryable while attempts remain,
                    // except request-construction and redirect-policy errors,
                    // which a retry cannot fix.
                    let retryable = !(err.is_builder() || err.is_redirect());
                    if retryable && attempt < max_retries {
                        tokio::time::sleep(retry_delay(
                            attempt,
                            &RetryAfter::Absent,
                            rand::random(),
                        ))
                        .await;
                        continue;
                    }
                    return Err(if err.is_timeout() {
                        OpenAIError::Timeout
                    } else if err.is_redirect() {
                        OpenAIError::Redirect(redirect_message(&err))
                    } else {
                        OpenAIError::Connection(err.to_string())
                    });
                }
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return Ok(response);
                    }

                    let headers = response.headers().clone();
                    let request_id = headers
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_owned);
                    let retry_after = parse_retry_after(&headers);
                    if attempt < max_retries
                        && should_retry(status.as_u16(), &headers, &retry_after)
                    {
                        tokio::time::sleep(retry_delay(attempt, &retry_after, rand::random()))
                            .await;
                        continue;
                    }

                    let body = response.text().await.unwrap_or_default();
                    return Err(OpenAIError::from_response(
                        status.as_u16(),
                        request_id,
                        &body,
                    ));
                }
            }
        }
        unreachable!("retry loop always returns")
    }

    /// Execute a request and deserialize the JSON response body.
    pub(crate) async fn execute<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        options: RequestOptions,
    ) -> Result<T, OpenAIError> {
        let response = self.execute_raw(method, path, options).await?;
        let bytes = response.bytes().await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Execute a request expected to return an empty body (e.g. a `DELETE`
    /// with `Accept: */*`), discarding any response bytes.
    pub(crate) async fn execute_no_content(
        &self,
        method: Method,
        path: &str,
        options: RequestOptions,
    ) -> Result<(), OpenAIError> {
        let response = self.execute_raw(method, path, options).await?;
        response.bytes().await?;
        Ok(())
    }

    /// Execute a request and return the raw response bytes (e.g. audio).
    pub(crate) async fn execute_bytes(
        &self,
        method: Method,
        path: &str,
        options: RequestOptions,
    ) -> Result<Bytes, OpenAIError> {
        let response = self.execute_raw(method, path, options).await?;
        Ok(response.bytes().await?)
    }

    /// Escape hatch: `GET` an arbitrary API path with query parameters.
    ///
    /// `T` may be [`serde_json::Value`] for untyped access.
    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, OpenAIError> {
        let query = query
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.execute(Method::GET, path, RequestOptions::query(query))
            .await
    }

    /// Escape hatch: `POST` an arbitrary API path with a JSON body.
    ///
    /// `T` may be [`serde_json::Value`] for untyped access.
    pub async fn post<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, OpenAIError> {
        let body = serde_json::to_value(body)?;
        self.execute(Method::POST, path, RequestOptions::json(body))
            .await
    }

    /// Escape hatch: `DELETE` an arbitrary API path.
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T, OpenAIError> {
        self.execute(Method::DELETE, path, RequestOptions::default())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_keeps_pchar_and_fine_tuned_ids() {
        assert_eq!(
            encode_path_segment("ft:gpt-4o:org::id").unwrap(),
            "ft:gpt-4o:org::id"
        );
        assert_eq!(
            encode_path_segment("a-b_c.d~e!$&'()*+,;=:@").unwrap(),
            "a-b_c.d~e!$&'()*+,;=:@"
        );
    }

    #[test]
    fn encode_escapes_reserved_and_unicode() {
        assert_eq!(encode_path_segment("org/model").unwrap(), "org%2Fmodel");
        assert_eq!(encode_path_segment("a?x=1").unwrap(), "a%3Fx=1");
        assert_eq!(encode_path_segment("a#b").unwrap(), "a%23b");
        assert_eq!(encode_path_segment("100%").unwrap(), "100%25");
        assert_eq!(encode_path_segment("a b").unwrap(), "a%20b");
        assert_eq!(encode_path_segment("é").unwrap(), "%C3%A9");
        assert_eq!(encode_path_segment("..x").unwrap(), "..x");
        assert_eq!(encode_path_segment("a/../b").unwrap(), "a%2F..%2Fb");
    }

    #[test]
    fn encode_rejects_dot_segments_and_empty() {
        for bad in ["", ".", "..", "%2e", "%2E%2e", ".%2E", "%2e."] {
            assert!(
                matches!(encode_path_segment(bad), Err(OpenAIError::Config(_))),
                "{bad:?} should be rejected"
            );
        }
        for ok in ["...", "%2e%2e%2e"] {
            assert!(encode_path_segment(ok).is_ok(), "{ok:?} should be accepted");
        }
    }
}
