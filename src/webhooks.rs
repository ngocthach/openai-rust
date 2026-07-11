//! Webhook signature verification, mirroring the `openai-python`
//! `resources/webhooks/webhooks.py` algorithm.
//!
//! OpenAI signs webhooks with an HMAC-SHA256 over the string
//! `"{webhook-id}.{webhook-timestamp}.{payload}"`, base64 (standard alphabet)
//! encoded. The `webhook-signature` header may carry several space-separated
//! signatures, each optionally prefixed with `"v1,"`; a request is accepted if
//! **any** of them matches, using a constant-time comparison.
//!
//! This module is standalone (no [`crate::Config`] dependency) so it can be
//! unit-tested in isolation and wired into the client in a later phase.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Default maximum age (and future skew) of a webhook, in seconds, mirroring
/// the Python SDK's `tolerance = 300`.
pub const DEFAULT_TOLERANCE_SECS: i64 = 300;

/// Errors produced while verifying a webhook signature.
///
/// This mirrors the messages raised by
/// `_exceptions.py::InvalidWebhookSignatureError` in `openai-python`.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WebhookVerificationError {
    /// One of the `webhook-id`/`webhook-timestamp`/`webhook-signature`
    /// headers is missing.
    #[error("Missing required webhook headers")]
    MissingHeaders,
    /// The `webhook-timestamp` header was not a valid integer.
    #[error("Invalid webhook timestamp format")]
    InvalidTimestampFormat,
    /// The webhook is older than the allowed tolerance (replay protection).
    #[error("Webhook timestamp is too old")]
    TimestampTooOld,
    /// The webhook timestamp is further in the future than the tolerance.
    #[error("Webhook timestamp is too new")]
    TimestampTooNew,
    /// None of the provided signatures matched the expected signature.
    #[error("The given webhook signature does not match the expected signature")]
    SignatureMismatch,
    /// The `whsec_` secret could not be base64-decoded.
    #[error("invalid webhook secret: {0}")]
    InvalidSecret(String),
    /// The payload could not be parsed as JSON (only used by [`Webhooks::unwrap`]).
    #[error("failed to parse webhook payload as JSON: {0}")]
    InvalidPayload(String),
}

/// The three headers required to verify a webhook, looked up case-insensitively
/// by the caller before constructing this struct:
/// `webhook-id`, `webhook-timestamp`, `webhook-signature`.
#[derive(Debug, Clone)]
pub struct WebhookHeaders {
    /// Value of the `webhook-id` header.
    pub id: String,
    /// Value of the `webhook-timestamp` header (kept as the original string;
    /// it is used verbatim when building the signed payload).
    pub timestamp: String,
    /// Value of the `webhook-signature` header (one or more space-separated
    /// signatures, each optionally prefixed with `"v1,"`).
    pub signature: String,
}

impl WebhookHeaders {
    /// Convenience constructor.
    pub fn new(
        id: impl Into<String>,
        timestamp: impl Into<String>,
        signature: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            timestamp: timestamp.into(),
            signature: signature.into(),
        }
    }

    /// Extract the three `webhook-*` headers via a lookup function, so any
    /// HTTP framework's header map plugs in without a dependency here:
    ///
    /// ```no_run
    /// # use openai_compat::webhooks::WebhookHeaders;
    /// # fn get_header(name: &str) -> Option<String> { None }
    /// let headers = WebhookHeaders::from_lookup(|name| get_header(name))
    ///     .expect("missing webhook headers");
    /// ```
    ///
    /// Lookups are performed with lowercase header names.
    pub fn from_lookup(
        get: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, WebhookVerificationError> {
        let field = |name: &str| get(name).ok_or(WebhookVerificationError::MissingHeaders);
        Ok(Self {
            id: field("webhook-id")?,
            timestamp: field("webhook-timestamp")?,
            signature: field("webhook-signature")?,
        })
    }
}

/// A verified webhook event envelope. The event body is kept as a raw
/// [`serde_json::Value`]; a fully typed union of event types is out of scope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebhookEvent {
    /// Unique identifier of the event.
    pub id: String,
    /// The event type, e.g. `response.completed`.
    #[serde(rename = "type")]
    pub r#type: String,
    /// Unix timestamp (seconds) of when the event was created.
    pub created_at: i64,
    /// The event payload, kept untyped.
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Verifies webhook signatures against a shared secret.
pub struct Webhooks {
    secret: Vec<u8>,
}

impl Webhooks {
    /// Create a new verifier from a webhook secret.
    ///
    /// If the secret starts with `"whsec_"`, the remainder is base64-decoded
    /// (standard alphabet) into the raw key bytes; otherwise the secret's raw
    /// bytes are used directly.
    ///
    /// Divergence from `webhooks.py`: the Python SDK decodes the secret lazily
    /// inside `verify_signature`; we decode eagerly here so an invalid
    /// `whsec_` secret is surfaced at construction time.
    pub fn new(secret: &str) -> Result<Self, WebhookVerificationError> {
        let secret = if let Some(rest) = secret.strip_prefix("whsec_") {
            STANDARD
                .decode(rest)
                .map_err(|e| WebhookVerificationError::InvalidSecret(e.to_string()))?
        } else {
            secret.as_bytes().to_vec()
        };
        Ok(Self { secret })
    }

    /// Verify the signature of a webhook payload.
    ///
    /// `now_unix` is the current time in Unix seconds (injected for testability;
    /// see [`Webhooks::verify`] for a variant that reads the system clock).
    /// `tolerance_secs` bounds how far in the past or future the webhook
    /// timestamp may be (default [`DEFAULT_TOLERANCE_SECS`]).
    pub fn verify_signature(
        &self,
        payload: &[u8],
        headers: &WebhookHeaders,
        tolerance_secs: i64,
        now_unix: i64,
    ) -> Result<(), WebhookVerificationError> {
        // 1. Parse and validate the timestamp (replay protection).
        let timestamp_seconds: i64 = headers
            .timestamp
            .trim()
            .parse()
            .map_err(|_| WebhookVerificationError::InvalidTimestampFormat)?;

        if now_unix - timestamp_seconds > tolerance_secs {
            return Err(WebhookVerificationError::TimestampTooOld);
        }
        if timestamp_seconds > now_unix + tolerance_secs {
            return Err(WebhookVerificationError::TimestampTooNew);
        }

        // 2. Extract candidate signatures ("v1,<base64>" or bare "<base64>"),
        //    split on ASCII whitespace.
        let signatures: Vec<&str> = headers
            .signature
            .split_whitespace()
            .map(|part| part.strip_prefix("v1,").unwrap_or(part))
            .collect();

        // 3. Compute the expected signature over "{id}.{timestamp}.{body}",
        //    feeding the payload as raw bytes (no lossy UTF-8 conversion).
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .expect("HMAC-SHA256 accepts keys of any length");
        mac.update(headers.id.as_bytes());
        mac.update(b".");
        mac.update(headers.timestamp.as_bytes());
        mac.update(b".");
        mac.update(payload);
        let expected = STANDARD.encode(mac.finalize().into_bytes());
        let expected_bytes = expected.as_bytes();

        // 4. Accept if any provided signature matches (constant-time).
        let matched = signatures.iter().any(|sig| {
            let sig_bytes = sig.as_bytes();
            // Guard length before the constant-time compare; unequal lengths
            // can never match.
            sig_bytes.len() == expected_bytes.len() && expected_bytes.ct_eq(sig_bytes).into()
        });

        if matched {
            Ok(())
        } else {
            Err(WebhookVerificationError::SignatureMismatch)
        }
    }

    /// Convenience wrapper that reads the current system time and uses the
    /// default tolerance.
    pub fn verify(
        &self,
        payload: &[u8],
        id: &str,
        timestamp: &str,
        signature_header: &str,
    ) -> Result<(), WebhookVerificationError> {
        let headers = WebhookHeaders::new(id, timestamp, signature_header);
        self.verify_signature(payload, &headers, DEFAULT_TOLERANCE_SECS, now_unix())
    }

    /// Verify the payload and parse it into a [`WebhookEvent`].
    ///
    /// Divergence from the parent shorthand: verification requires the webhook
    /// headers, so `unwrap` takes them explicitly (the Python `unwrap(payload,
    /// headers, secret)` does the same). Uses the system clock and default
    /// tolerance.
    pub fn unwrap(
        &self,
        payload: &[u8],
        headers: &WebhookHeaders,
    ) -> Result<WebhookEvent, WebhookVerificationError> {
        self.verify_signature(payload, headers, DEFAULT_TOLERANCE_SECS, now_unix())?;
        serde_json::from_slice(payload)
            .map_err(|e| WebhookVerificationError::InvalidPayload(e.to_string()))
    }
}

/// Current time in Unix seconds. Falls back to 0 if the system clock is set
/// before the epoch (which makes every timestamp check fail closed).
fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compute a valid signature for a payload the same way the server would,
    /// so tests can self-check without hard-coding opaque base64 blobs.
    fn sign(secret_bytes: &[u8], id: &str, timestamp: &str, payload: &[u8]) -> String {
        let body = String::from_utf8_lossy(payload);
        let signed = format!("{id}.{timestamp}.{body}");
        let mut mac = HmacSha256::new_from_slice(secret_bytes).unwrap();
        mac.update(signed.as_bytes());
        STANDARD.encode(mac.finalize().into_bytes())
    }

    const SECRET: &str = "my-webhook-secret";
    const ID: &str = "wh_123";
    const PAYLOAD: &[u8] =
        br#"{"id":"evt_1","type":"response.completed","created_at":100,"data":{"foo":"bar"}}"#;

    #[test]
    fn accepts_valid_signature() {
        let ts = "1000";
        let sig = sign(SECRET.as_bytes(), ID, ts, PAYLOAD);
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, ts, format!("v1,{sig}"));
        // now within tolerance of ts.
        assert!(wh
            .verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1000)
            .is_ok());
    }

    #[test]
    fn accepts_bare_signature_without_v1_prefix() {
        let ts = "1000";
        let sig = sign(SECRET.as_bytes(), ID, ts, PAYLOAD);
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, ts, sig); // no "v1," prefix
        assert!(wh
            .verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1000)
            .is_ok());
    }

    #[test]
    fn rejects_wrong_signature() {
        let ts = "1000";
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, ts, "v1,not-the-right-signature");
        assert_eq!(
            wh.verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1000),
            Err(WebhookVerificationError::SignatureMismatch)
        );
    }

    #[test]
    fn rejects_signature_from_wrong_secret() {
        let ts = "1000";
        let sig = sign(b"a-different-secret", ID, ts, PAYLOAD);
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, ts, format!("v1,{sig}"));
        assert_eq!(
            wh.verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1000),
            Err(WebhookVerificationError::SignatureMismatch)
        );
    }

    #[test]
    fn rejects_tampered_payload() {
        let ts = "1000";
        let sig = sign(SECRET.as_bytes(), ID, ts, PAYLOAD);
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, ts, format!("v1,{sig}"));
        let tampered =
            br#"{"id":"evt_1","type":"response.completed","created_at":100,"data":{"foo":"BAZ"}}"#;
        assert_eq!(
            wh.verify_signature(tampered, &headers, DEFAULT_TOLERANCE_SECS, 1000),
            Err(WebhookVerificationError::SignatureMismatch)
        );
    }

    #[test]
    fn rejects_expired_timestamp() {
        let ts = "1000";
        let sig = sign(SECRET.as_bytes(), ID, ts, PAYLOAD);
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, ts, format!("v1,{sig}"));
        // now is 1000 + 301 -> older than tolerance 300.
        assert_eq!(
            wh.verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1301),
            Err(WebhookVerificationError::TimestampTooOld)
        );
    }

    #[test]
    fn rejects_future_timestamp() {
        let ts = "1000";
        let sig = sign(SECRET.as_bytes(), ID, ts, PAYLOAD);
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, ts, format!("v1,{sig}"));
        // ts is 301 seconds ahead of now -> too new.
        assert_eq!(
            wh.verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 699),
            Err(WebhookVerificationError::TimestampTooNew)
        );
    }

    #[test]
    fn rejects_non_integer_timestamp() {
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, "not-a-number", "v1,whatever");
        assert_eq!(
            wh.verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1000),
            Err(WebhookVerificationError::InvalidTimestampFormat)
        );
    }

    #[test]
    fn decodes_whsec_prefixed_secret() {
        // Raw key bytes, then encode into a whsec_ secret.
        let raw_key = b"raw-secret-bytes-32-chars-long!!";
        let whsec = format!("whsec_{}", STANDARD.encode(raw_key));
        let ts = "1000";
        let sig = sign(raw_key, ID, ts, PAYLOAD);

        let wh = Webhooks::new(&whsec).unwrap();
        let headers = WebhookHeaders::new(ID, ts, format!("v1,{sig}"));
        assert!(wh
            .verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1000)
            .is_ok());
    }

    #[test]
    fn invalid_whsec_secret_is_rejected_at_construction() {
        let result = Webhooks::new("whsec_!!!not base64!!!");
        assert!(matches!(
            result,
            Err(WebhookVerificationError::InvalidSecret(_))
        ));
    }

    #[test]
    fn accepts_when_only_second_of_multiple_signatures_matches() {
        let ts = "1000";
        let good = sign(SECRET.as_bytes(), ID, ts, PAYLOAD);
        let wh = Webhooks::new(SECRET).unwrap();
        // First signature is garbage, second is valid; space-separated.
        let header = format!("v1,aGVsbG8gd29ybGQ v1,{good}");
        let headers = WebhookHeaders::new(ID, ts, header);
        assert!(wh
            .verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1000)
            .is_ok());
    }

    #[test]
    fn unwrap_verifies_then_parses() {
        let ts = "1000";
        let sig = sign(SECRET.as_bytes(), ID, ts, PAYLOAD);
        let wh = Webhooks::new(SECRET).unwrap();
        let headers = WebhookHeaders::new(ID, ts, format!("v1,{sig}"));

        // unwrap uses the system clock, so use a fresh timestamp for this test.
        let now = now_unix().to_string();
        let sig_now = sign(SECRET.as_bytes(), ID, &now, PAYLOAD);
        let headers_now = WebhookHeaders::new(ID, &now, format!("v1,{sig_now}"));
        let event = wh.unwrap(PAYLOAD, &headers_now).unwrap();
        assert_eq!(event.id, "evt_1");
        assert_eq!(event.r#type, "response.completed");
        assert_eq!(event.created_at, 100);
        assert_eq!(event.data["foo"], "bar");

        // Sanity: the fixed-time verifier still works with injected now.
        assert!(wh
            .verify_signature(PAYLOAD, &headers, DEFAULT_TOLERANCE_SECS, 1000)
            .is_ok());
    }
}
