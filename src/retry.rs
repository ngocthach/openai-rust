//! Pure retry helpers mirroring `_base_client.py:737-828`.
//!
//! Kept free of I/O so the exact backoff math is unit-testable.

use std::time::Duration;

use reqwest::header::HeaderMap;

/// From `_constants.py`.
const INITIAL_RETRY_DELAY: f64 = 0.5;
const MAX_RETRY_DELAY: f64 = 8.0;

/// Parse the `retry-after-ms` (preferred, milliseconds) or `retry-after`
/// (seconds, nonstandard floats allowed) headers into seconds.
///
/// The HTTP-date form of `retry-after` is not supported (rare in practice;
/// falls back to exponential backoff).
pub(crate) fn parse_retry_after(headers: &HeaderMap) -> Option<f64> {
    if let Some(ms) = headers
        .get("retry-after-ms")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<f64>().ok())
    {
        return Some(ms / 1000.0);
    }
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<f64>().ok())
}

/// Decide whether an error response should be retried, mirroring
/// `_base_client.py::_should_retry`: the `x-should-retry` header wins,
/// otherwise 408/409/429/5xx are retryable.
pub(crate) fn should_retry(status: u16, headers: &HeaderMap) -> bool {
    match headers.get("x-should-retry").and_then(|v| v.to_str().ok()) {
        Some("true") => return true,
        Some("false") => return false,
        _ => {}
    }
    matches!(status, 408 | 409 | 429) || status >= 500
}

/// Compute the backoff before retry number `nb_retries` (0-based count of
/// retries already taken), mirroring `_base_client.py::_calculate_retry_timeout`.
///
/// `rand01` must be uniform in `[0, 1)`; it is injected so tests are
/// deterministic. Production passes `rand::random()`.
pub(crate) fn calculate_retry_timeout(
    nb_retries: u32,
    retry_after: Option<f64>,
    rand01: f64,
) -> Duration {
    // If the API asks us to wait a reasonable amount of time, obey it.
    if let Some(retry_after) = retry_after {
        if retry_after > 0.0 && retry_after <= 60.0 {
            return Duration::from_secs_f64(retry_after);
        }
    }

    let nb_retries = nb_retries.min(1000);
    let sleep_seconds = (INITIAL_RETRY_DELAY * 2.0_f64.powi(nb_retries as i32)).min(MAX_RETRY_DELAY);

    // Jitter: 0.75x..1.0x of the computed delay.
    let jitter = 1.0 - 0.25 * rand01;
    let timeout = sleep_seconds * jitter;
    Duration::from_secs_f64(timeout.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        map
    }

    #[test]
    fn retry_after_ms_preferred_over_seconds() {
        let h = headers(&[("retry-after-ms", "1500"), ("retry-after", "99")]);
        assert_eq!(parse_retry_after(&h), Some(1.5));
    }

    #[test]
    fn retry_after_seconds_and_floats() {
        assert_eq!(parse_retry_after(&headers(&[("retry-after", "2")])), Some(2.0));
        assert_eq!(parse_retry_after(&headers(&[("retry-after", "0.75")])), Some(0.75));
        // HTTP-date form unsupported -> None
        assert_eq!(
            parse_retry_after(&headers(&[("retry-after", "Fri, 04 Jul 2026 00:00:00 GMT")])),
            None
        );
        assert_eq!(parse_retry_after(&HeaderMap::new()), None);
    }

    #[test]
    fn should_retry_honors_header_override() {
        assert!(should_retry(400, &headers(&[("x-should-retry", "true")])));
        assert!(!should_retry(500, &headers(&[("x-should-retry", "false")])));
    }

    #[test]
    fn should_retry_status_codes() {
        let h = HeaderMap::new();
        for status in [408, 409, 429, 500, 502, 503, 599] {
            assert!(should_retry(status, &h), "expected retry for {status}");
        }
        for status in [400, 401, 403, 404, 422, 418] {
            assert!(!should_retry(status, &h), "expected no retry for {status}");
        }
    }

    #[test]
    fn backoff_doubles_and_caps() {
        // rand01 = 0 -> jitter factor 1.0 (no jitter)
        assert_eq!(calculate_retry_timeout(0, None, 0.0), Duration::from_secs_f64(0.5));
        assert_eq!(calculate_retry_timeout(1, None, 0.0), Duration::from_secs_f64(1.0));
        assert_eq!(calculate_retry_timeout(2, None, 0.0), Duration::from_secs_f64(2.0));
        assert_eq!(calculate_retry_timeout(3, None, 0.0), Duration::from_secs_f64(4.0));
        assert_eq!(calculate_retry_timeout(4, None, 0.0), Duration::from_secs_f64(8.0));
        assert_eq!(calculate_retry_timeout(10, None, 0.0), Duration::from_secs_f64(8.0));
    }

    #[test]
    fn jitter_scales_down_by_up_to_a_quarter() {
        let full = calculate_retry_timeout(1, None, 0.0);
        let jittered = calculate_retry_timeout(1, None, 1.0);
        assert_eq!(jittered.as_secs_f64(), full.as_secs_f64() * 0.75);
    }

    #[test]
    fn retry_after_used_when_reasonable() {
        assert_eq!(
            calculate_retry_timeout(0, Some(2.0), 0.5),
            Duration::from_secs_f64(2.0)
        );
        // > 60s is ignored, falls back to backoff
        assert_eq!(
            calculate_retry_timeout(0, Some(120.0), 0.0),
            Duration::from_secs_f64(0.5)
        );
        // <= 0 ignored
        assert_eq!(
            calculate_retry_timeout(0, Some(0.0), 0.0),
            Duration::from_secs_f64(0.5)
        );
    }
}
