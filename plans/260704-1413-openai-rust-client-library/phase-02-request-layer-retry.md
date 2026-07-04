# Phase 02 — Request Layer + Retry Logic

## Context Links
- Reference: `openai-python/src/openai/_base_client.py:737-793` (`_parse_retry_after_header`, `_calculate_retry_timeout`)
- Reference: `_base_client.py:795-828` (`_should_retry`)
- Reference: `_base_client.py:456-480` (header building, `x-stainless-retry-count`)
- Reference: `_base_client.py:1006-1128` (request loop, sleep-for-retry, error wrapping)

## Overview
- **Priority:** P1
- **Status:** pending
- Core `execute()` request pipeline: build request, send, on failure decide retry, backoff+sleep, map errors. All resources call through this. Also generic `get`/`post` escape hatch.

## Key Insights
- Retry loop (`_base_client.py:1017`): iterate `retries_taken` in `0..=max_retries`, `remaining = max_retries - retries_taken`.
- Retry conditions: connection error / timeout (`APIConnectionError`/`APITimeoutError`) always retry if remaining>0; status errors retry only if `remaining>0 && _should_retry(response)`.
- `_should_retry` (`:795`): honor `x-should-retry` header (`"true"`→retry, `"false"`→no); else 408, 409, 429, `>=500`.
- Timeout calc (`:771-793`):
  - If `retry-after-ms` header present → `ms/1000`; else `retry-after` seconds (float) or HTTP-date.
  - If parsed `retry_after` and `0 < retry_after <= 60` → use it directly.
  - Else `nb_retries = min(max_retries - remaining_retries, 1000)`; `sleep = min(0.5 * 2^nb_retries, 8.0)`; `jitter = 1 - 0.25*rand()`; `timeout = sleep*jitter` (clamp >=0).
- Header `x-stainless-retry-count` set per attempt (`:471-472`). We can send equivalent (optional, keep for parity).

## Requirements
**Functional**
- `Client::execute<T: DeserializeOwned>(method, path, RequestOptions) -> Result<T, OpenAIError>`.
- `RequestOptions { query, json_body, multipart, extra_headers, timeout_override }`.
- Retry engine honoring all rules above; `tokio::time::sleep` for backoff.
- Raw variants: `execute_bytes` (binary, e.g. audio speech) and `execute_stream` (returns response for phase 03).
- Generic escape hatch: `client.get<T>(path, query)`, `client.post<B: Serialize, T>(path, &body)`.

**Non-functional**
- Retry sleep uses jitter; deterministic-ish for tests via seam (inject delay or expose `calculate_retry_timeout` as testable pure fn).

## Architecture
```
src/
├── request.rs        # RequestOptions, execute pipeline, generic get/post
└── retry.rs          # should_retry(status, headers), parse_retry_after, calculate_retry_timeout (pure fns → unit-testable)
```
Keep `retry.rs` pure (inputs: status, headers map, attempt, max_retries, rand source) so unit tests assert exact backoff math.

## Related Code Files
**Create:** `src/request.rs`, `src/retry.rs`
**Modify:** `src/client.rs` (add execute methods, wire resource accessors), `src/lib.rs` (re-export RequestOptions if public), `src/error.rs` (map reqwest timeout→`Timeout`, connect→`Connection`).

## Implementation Steps
1. `retry.rs`:
   - `fn parse_retry_after(headers: &HeaderMap) -> Option<f64>`: try `retry-after-ms` (→ /1000), then `retry-after` as f64, then HTTP-date (use `httpdate` crate OR skip date parsing — KISS: support ms + seconds only, note in code that date form is rare). **Decision:** support `retry-after-ms` and numeric `retry-after`; skip date parsing for v0.1 (document).
   - `fn should_retry(status: u16, headers: &HeaderMap) -> bool`: `x-should-retry` override; else 408/409/429/>=500.
   - `fn calculate_retry_timeout(attempt: u32, max_retries: u32, retry_after: Option<f64>, rand01: f64) -> Duration`: implement exact formula. `rand01` injected for tests; production passes `rand::random::<f64>()`.
2. `request.rs`:
   - Build `reqwest::RequestBuilder`: method + `base_url + path`, auth + org/project + default + extra headers, query, body (json or multipart), per-request timeout.
   - Loop `for attempt in 0..=max_retries`:
     - `send().await`: on reqwest error → classify timeout vs connect; if `attempt < max_retries` sleep(calculate_retry_timeout(attempt, .., None, rand)) and continue; else return `Timeout`/`Connection`.
     - On response: if `status.is_success()` → read + deserialize `T` (for `()` return, skip). If error status → read body + `x-request-id`; if `attempt < max_retries && should_retry(status,&headers)` → sleep(calculate_retry_timeout(attempt,.., parse_retry_after(&headers), rand)) and continue; else `return Err(error::from_response(...))`.
   - `execute_bytes`: same loop but return `bytes::Bytes` on success.
   - `execute_raw` (for streaming): return the `reqwest::Response` on success (no body read), same retry loop on the initial send/status.
   - Generic `get`/`post`: thin wrappers over `execute`.
3. `client.rs`: expose `pub(crate)` execute methods; make resource accessors return resource structs holding `&Client` / cloned `Client`.
4. `cargo build`.

## Todo List
- [ ] retry.rs: parse_retry_after (ms + seconds)
- [ ] retry.rs: should_retry (header override + status codes)
- [ ] retry.rs: calculate_retry_timeout (exact backoff + injected jitter)
- [ ] request.rs: execute<T> with retry loop + error mapping
- [ ] request.rs: execute_bytes, execute_raw (for stream)
- [ ] request.rs: generic get/post escape hatch
- [ ] cargo build clean

## Success Criteria
- Non-retryable (400/401/403/404/422) returns immediately, no sleep.
- Retryable (429/500/503/connection) retried up to `max_retries`, then surfaces error.
- `retry-after: 2` → sleeps ~2s (bypasses backoff); `retry-after: 120` → ignored (>60), uses backoff.
- `calculate_retry_timeout` matches `0.5*2^n` capped at 8.0 (unit-tested with fixed rand01).

## Risk Assessment
- reqwest error classification: use `err.is_timeout()` / `err.is_connect()`. Mitigation: fallback to `Connection` for ambiguous.
- Reading body once: must buffer error body before deciding retry. Mitigation: `resp.text().await` on error path.

## Security Considerations
- Do not include api_key in any error message or retry log.

## Next Steps
- Phase 03 uses `execute_raw` to obtain the streaming `reqwest::Response`.
