# Phase 01 — Scaffolding, Config, Client Core, Error Hierarchy

## Context Links
- Reference: `openai-python/src/openai/_client.py` (config, auth headers, env vars)
- Reference: `openai-python/src/openai/_constants.py` (defaults)
- Reference: `openai-python/src/openai/_exceptions.py` (error hierarchy)
- Reference: `openai-python/src/openai/_base_client.py:407-436` (`_make_status_error_from_response`, `_make_status_error`)

## Overview
- **Priority:** P1 (foundation for all other phases)
- **Status:** pending
- Set up the crate, `Config`/`Client` builder, and the full error hierarchy. No HTTP calls yet — just types, config resolution, and error mapping.

## Key Insights
- Env fallbacks (`_client.py:209-252`): `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_ORG_ID`, `OPENAI_PROJECT_ID`. Default base_url `https://api.openai.com/v1`.
- Auth header (`_client.py:502`): `Authorization: Bearer {api_key}`. Org/project (`_client.py:528-529`): `OpenAI-Organization`, `OpenAI-Project` (omit when None).
- Error status mapping (`_exceptions.py:117-163`): 400→BadRequest, 401→Authentication, 403→PermissionDenied, 404→NotFound, 409→Conflict, 422→UnprocessableEntity, 429→RateLimit, >=500→InternalServer, other 4xx→generic ApiStatus.
- Error body parse (`_exceptions.py:73-80`): body may be `{"error": {message, type, param, code}}` or raw string. `request_id` from `x-request-id` header (`_exceptions.py:104`).
- Defaults: timeout 600s, max_retries 2.

## Requirements
**Functional**
- `Config` holds: api_key, base_url, organization, project, timeout, max_retries, default_headers.
- `Client::builder()` → `ClientBuilder` with setters; `.build()` resolves env fallbacks + validates api_key present.
- `OpenAIError` enum covering all variants + serde-parsed `ApiErrorBody { message, r#type, param, code }`.
- Function to map `(status, headers, body)` → correct `OpenAIError` variant with `request_id`.

**Non-functional**
- Each source file < 800 lines. No panics on missing env — return `OpenAIError::Config`.

## Architecture
```
src/
├── lib.rs            # re-exports, crate docs
├── config.rs         # Config, ClientBuilder
├── client.rs         # Client (holds reqwest::Client + Config); resource accessors (stubs)
└── error.rs          # OpenAIError, ApiErrorBody, status→variant mapping, ApiErrorDetail
```
`Client` wraps `Arc<Inner>` (reqwest::Client + resolved Config) so it is cheaply `Clone`.

## Related Code Files
**Create**
- `Cargo.toml`, `src/lib.rs`, `src/config.rs`, `src/client.rs`, `src/error.rs`
- `.gitignore` (add `/target`, `Cargo.lock` kept for bin? — library: ignore lock optional)

## Cargo.toml Dependencies (exact)
```toml
[package]
name = "openai-rust"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT"

[dependencies]
reqwest = { version = "0.12", default-features = false, features = ["json", "stream", "rustls-tls", "multipart"] }
tokio = { version = "1", features = ["rt", "time", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
futures = "0.3"
thiserror = "2"
bytes = "1"
rand = "0.8"

[dev-dependencies]
wiremock = "0.6"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "test-util"] }
tokio-test = "0.4"
```

## Implementation Steps
1. `cargo init --lib`; fill `Cargo.toml` above.
2. `error.rs`: define
   - `ApiErrorDetail { message: Option<String>, r#type: Option<String>, param: Option<String>, code: Option<String> }` (`#[derive(Deserialize)]`).
   - `enum OpenAIError { Config(String), Connection(String), Timeout, Api(ApiError), Stream(String), Json(serde_json::Error), Http(reqwest::Error) }` via `thiserror`.
   - `struct ApiError { status: u16, kind: ApiErrorKind, message: String, detail: Option<ApiErrorDetail>, request_id: Option<String> }`.
   - `enum ApiErrorKind { BadRequest, Authentication, PermissionDenied, NotFound, Conflict, UnprocessableEntity, RateLimit, InternalServer, Other }`.
   - `fn from_response(status: u16, request_id: Option<String>, body: &str) -> OpenAIError`: parse `{"error": {...}}` → detail; build message `Error code: {status} - {body}` mirroring `_base_client.py:423`; map status → kind mirroring `_should_retry`/`_exceptions`.
   - Helper `ApiError::is_retryable()` (408/409/429/>=500) for phase 02.
3. `config.rs`: `Config` struct + defaults consts (`DEFAULT_TIMEOUT_SECS=600`, `DEFAULT_MAX_RETRIES=2`, `DEFAULT_BASE_URL`). `ClientBuilder` with `api_key`, `base_url`, `organization`, `project`, `timeout`, `max_retries`, `header(k,v)`. `.build()` applies env fallbacks (std::env), trims trailing `/` on base_url, errors if api_key empty.
4. `client.rs`: `Client` constructs `reqwest::ClientBuilder` (timeout, no built-in retry). Store base_url, auth headers computed once. Add stub methods `chat()`, `embeddings()`, etc. returning resource structs (filled later) — or defer accessors to their phases; keep placeholders compiling.
5. `lib.rs`: re-export `Client`, `Config`, `OpenAIError`, `ApiError*`. Crate-level `//!` doc with a minimal usage example (doctest `no_run`).
6. `cargo build` — must compile clean.

## Todo List
- [ ] `cargo init --lib` + Cargo.toml deps
- [ ] error.rs: OpenAIError + ApiError + status mapping + from_response
- [ ] config.rs: Config + ClientBuilder + env fallbacks
- [ ] client.rs: Client (Arc inner, reqwest client, auth headers)
- [ ] lib.rs re-exports + crate doc
- [ ] `cargo build` clean

## Success Criteria
- `cargo build` succeeds. Builder resolves env vars; missing api_key → `OpenAIError::Config`.
- `from_response` maps each status to the correct `ApiErrorKind` and extracts `request_id`.

## Risk Assessment
- reqwest feature flags: ensure `rustls-tls` (not default openssl) to avoid system OpenSSL dependency. Mitigation: `default-features = false`.

## Security Considerations
- Never log api_key. `Config` Debug impl must redact api_key.

## Next Steps
- Phase 02 consumes `Config`, `ApiError::is_retryable`, and the reqwest client.
