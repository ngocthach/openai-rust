//! Pure retry helpers mirroring `_base_client.py:737-828`.
//!
//! Kept free of I/O so the exact backoff math is unit-testable.

use std::time::Duration;

use reqwest::header::HeaderMap;

/// From `_constants.py`.
const INITIAL_RETRY_DELAY: f64 = 0.5;
const MAX_RETRY_DELAY: f64 = 8.0;

/// Longest server-directed wait (seconds) that is honored; a larger
/// `retry-after` means the request is not retried at all.
const MAX_RETRY_AFTER_DELAY_SECS: f64 = 120.0;

/// Interpretation of the `retry-after-ms` / `retry-after` response headers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RetryAfter {
    /// No usable header: fall back to exponential backoff.
    Absent,
    /// Server-directed wait in seconds; always finite and in `(0, 120]`.
    Delay(f64),
    /// The server asked for a wait beyond the maximum: do not retry.
    Refuse,
}

/// Parse the `retry-after-ms` (preferred, milliseconds) or `retry-after`
/// (seconds, nonstandard floats allowed) headers.
///
/// The first header whose value parses as a float decides the outcome, even
/// when that value is unusable (NaN, negative, zero, a literal `inf`); later
/// headers are not consulted. A finite value above 120s and a numeric
/// overflow such as `1e400` both mean [`RetryAfter::Refuse`], whereas a
/// literal `inf` is treated as a malformed header and ignored.
///
/// The HTTP-date form of `retry-after` is not supported (rare in practice;
/// falls back to exponential backoff).
pub(crate) fn parse_retry_after(headers: &HeaderMap) -> RetryAfter {
    for (name, divisor) in [("retry-after-ms", 1000.0), ("retry-after", 1.0)] {
        let Some(raw) = headers.get(name).and_then(|v| v.to_str().ok()) else {
            continue;
        };
        let raw = raw.trim();
        let Ok(value) = raw.parse::<f64>() else {
            continue;
        };
        if value == f64::INFINITY {
            let literal = matches!(
                raw.to_ascii_lowercase().as_str(),
                "inf" | "+inf" | "infinity" | "+infinity"
            );
            return if literal {
                RetryAfter::Absent
            } else {
                RetryAfter::Refuse
            };
        }
        let seconds = value / divisor;
        return if !seconds.is_finite() || seconds <= 0.0 {
            RetryAfter::Absent
        } else if seconds > MAX_RETRY_AFTER_DELAY_SECS {
            RetryAfter::Refuse
        } else {
            RetryAfter::Delay(seconds)
        };
    }
    RetryAfter::Absent
}

/// Decide whether an error response should be retried, mirroring
/// `_base_client.py::_should_retry`: an excessive `retry-after` forbids any
/// retry, then the `x-should-retry` header wins, otherwise 408/409/429/5xx
/// are retryable.
pub(crate) fn should_retry(status: u16, headers: &HeaderMap, retry_after: &RetryAfter) -> bool {
    if *retry_after == RetryAfter::Refuse {
        return false;
    }
    match headers.get("x-should-retry").and_then(|v| v.to_str().ok()) {
        Some("true") => return true,
        Some("false") => return false,
        _ => {}
    }
    matches!(status, 408 | 409 | 429) || status >= 500
}

/// Compute the wait before retry number `nb_retries` (0-based count of
/// retries already taken), mirroring `_base_client.py::_calculate_retry_timeout`.
///
/// `rand01` must be uniform in `[0, 1)`; it is injected so tests are
/// deterministic. Production passes `rand::random()`.
pub(crate) fn retry_delay(nb_retries: u32, retry_after: &RetryAfter, rand01: f64) -> Duration {
    // If the API asks us to wait a reasonable amount of time, obey it.
    // `Delay` is finite and positive by construction, so this cannot panic.
    if let RetryAfter::Delay(seconds) = retry_after {
        return Duration::from_secs_f64(*seconds);
    }

    let nb_retries = nb_retries.min(1000);
    let sleep_seconds =
        (INITIAL_RETRY_DELAY * 2.0_f64.powi(nb_retries as i32)).min(MAX_RETRY_DELAY);

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

    fn parsed(pairs: &[(&str, &str)]) -> RetryAfter {
        parse_retry_after(&headers(pairs))
    }

    #[test]
    fn retry_after_seconds_table() {
        use RetryAfter::*;
        let cases: &[(&str, RetryAfter)] = &[
            ("2", Delay(2.0)),
            ("0.75", Delay(0.75)),
            ("90", Delay(90.0)),
            ("120", Delay(120.0)),
            ("121", Refuse),
            ("300", Refuse),
            ("0", Absent),
            ("-1", Absent),
            ("NaN", Absent),
            ("inf", Absent),
            ("+Inf", Absent),
            ("Infinity", Absent),
            ("+INFINITY", Absent),
            ("-inf", Absent),
            ("1e400", Refuse),
            ("garbage", Absent),
            ("", Absent),
            // HTTP-date form is unsupported and falls back to backoff.
            ("Fri, 04 Jul 2099 00:00:00 GMT", Absent),
        ];
        for (value, expected) in cases {
            assert_eq!(
                parsed(&[("retry-after", value)]),
                *expected,
                "retry-after: {value:?}"
            );
        }
    }

    #[test]
    fn retry_after_ms_table() {
        use RetryAfter::*;
        let cases: &[(&str, RetryAfter)] = &[
            ("1500", Delay(1.5)),
            ("120000", Delay(120.0)),
            ("120001", Refuse),
            ("0", Absent),
            ("-5", Absent),
            ("NaN", Absent),
            ("inf", Absent),
            ("1e400", Refuse),
        ];
        for (value, expected) in cases {
            assert_eq!(
                parsed(&[("retry-after-ms", value)]),
                *expected,
                "retry-after-ms: {value:?}"
            );
        }
    }

    #[test]
    fn missing_headers_are_absent() {
        assert_eq!(parse_retry_after(&HeaderMap::new()), RetryAfter::Absent);
    }

    #[test]
    fn retry_after_ms_preferred_over_seconds() {
        assert_eq!(
            parsed(&[("retry-after-ms", "1500"), ("retry-after", "99")]),
            RetryAfter::Delay(1.5)
        );
    }

    #[test]
    fn first_parseable_header_decides_without_fallthrough() {
        // An unusable ms value does not defer to a long seconds value.
        assert_eq!(
            parsed(&[("retry-after-ms", "inf"), ("retry-after", "300")]),
            RetryAfter::Absent
        );
        assert_eq!(
            parsed(&[("retry-after-ms", "0"), ("retry-after", "300")]),
            RetryAfter::Absent
        );
        // An unparseable ms value does fall through to seconds.
        assert_eq!(
            parsed(&[("retry-after-ms", "soon"), ("retry-after", "3")]),
            RetryAfter::Delay(3.0)
        );
    }

    #[test]
    fn refuse_overrides_should_retry_header() {
        let h = headers(&[("x-should-retry", "true")]);
        assert!(!should_retry(429, &h, &RetryAfter::Refuse));
        assert!(should_retry(429, &h, &RetryAfter::Delay(3.0)));
    }

    #[test]
    fn should_retry_honors_header_override() {
        let absent = RetryAfter::Absent;
        assert!(should_retry(
            400,
            &headers(&[("x-should-retry", "true")]),
            &absent
        ));
        assert!(!should_retry(
            500,
            &headers(&[("x-should-retry", "false")]),
            &absent
        ));
    }

    #[test]
    fn should_retry_status_codes() {
        let h = HeaderMap::new();
        for status in [408, 409, 429, 500, 502, 503, 599] {
            assert!(
                should_retry(status, &h, &RetryAfter::Absent),
                "expected retry for {status}"
            );
        }
        for status in [400, 401, 403, 404, 422, 418] {
            assert!(
                !should_retry(status, &h, &RetryAfter::Absent),
                "expected no retry for {status}"
            );
        }
    }

    #[test]
    fn backoff_doubles_and_caps() {
        // rand01 = 0 -> jitter factor 1.0 (no jitter)
        assert_eq!(
            retry_delay(0, &RetryAfter::Absent, 0.0),
            Duration::from_secs_f64(0.5)
        );
        assert_eq!(
            retry_delay(1, &RetryAfter::Absent, 0.0),
            Duration::from_secs_f64(1.0)
        );
        assert_eq!(
            retry_delay(2, &RetryAfter::Absent, 0.0),
            Duration::from_secs_f64(2.0)
        );
        assert_eq!(
            retry_delay(3, &RetryAfter::Absent, 0.0),
            Duration::from_secs_f64(4.0)
        );
        assert_eq!(
            retry_delay(4, &RetryAfter::Absent, 0.0),
            Duration::from_secs_f64(8.0)
        );
        assert_eq!(
            retry_delay(10, &RetryAfter::Absent, 0.0),
            Duration::from_secs_f64(8.0)
        );
    }

    #[test]
    fn jitter_scales_down_by_up_to_a_quarter() {
        let full = retry_delay(1, &RetryAfter::Absent, 0.0);
        let jittered = retry_delay(1, &RetryAfter::Absent, 1.0);
        assert_eq!(jittered.as_secs_f64(), full.as_secs_f64() * 0.75);
    }

    #[test]
    fn retry_after_delay_is_honored_up_to_the_cap() {
        assert_eq!(
            retry_delay(0, &RetryAfter::Delay(2.0), 0.5),
            Duration::from_secs_f64(2.0)
        );
        assert_eq!(
            retry_delay(0, &RetryAfter::Delay(90.0), 0.0),
            Duration::from_secs(90)
        );
        assert_eq!(
            retry_delay(0, &RetryAfter::Delay(120.0), 0.0),
            Duration::from_secs(120)
        );
    }

    #[test]
    fn absent_retry_after_uses_backoff() {
        assert_eq!(
            retry_delay(0, &RetryAfter::Absent, 0.0),
            Duration::from_secs_f64(0.5)
        );
    }
}
