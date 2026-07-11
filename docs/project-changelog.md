# Project Changelog

## Unreleased — CI/doc hardening (2026-07-11)

- **Added** `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` steps to `.github/workflows/rust.yml`, so the "clippy clean" claim in every prior release entry is now enforced by CI instead of only checked manually before publish.
- **Fixed** `cargo fmt --check` violations across 22 src/test/example files (pure formatting, no behavior change) so CI starts green.
- **Fixed** README install snippet still pinned `openai-compat = "0.2"` after the v0.3.0 release.
- **Fixed** `docs/development-roadmap.md` still listing the Responses API under "Future (unscheduled)" after it shipped in v0.3.0; added the missing "Phase 3" entry.

No crate code behavior changed; no version bump or republish.

## v0.3.0 — 2026-07-05

- **Added** the Responses API (`/v1/responses`) as a new `client.responses()` resource: `create`/`create_stream`, `retrieve`/`retrieve_stream` (background-mode resumable streaming via `starting_after`), `delete`, `cancel`, and `input_items().list()/list_all()` cursor pagination.
- **Added** typed request/response surface in `src/types/responses/`: `CreateResponseRequest` builder (`text`/`reasoning`/`include`/`truncation`/`previous_response_id`/`background`/...), `Input`/`InputItem` (message, function_call, function_call_output, reasoning, item_reference — other item types preserved via `Other(Value)`), `Tool`/`ToolChoice` (function fully typed; `web_search`/`file_search`/`code_interpreter` thinly typed; other built-ins via `Other(Value)`), `Response`/`OutputItem` with `output_text()`/`function_calls()` helpers.
- **Added** `ResponseStreamEvent`: ~12 typed streaming event variants (`Created`, `InProgress`, `OutputTextDelta`, `FunctionCallArgumentsDelta`, `Completed`, `Failed`, `Incomplete`, ...) plus a forward-compatible `Unknown` catch-all for the ~40 event types not modeled. `Failed`/`Incomplete` are typed `Ok` variants, not stream errors, since their error detail nests under `response.error` rather than the top-level `{"error":...}` shape the SSE decoder already intercepts.
- **Added** `execute_no_content` helper in `request.rs` for the empty-body `DELETE /responses/{id}` response (additive-only; no other resource's request path changed).
- **Added** `examples/responses.rs`, README "Responses API" section, and a new `tests/responses-tests.rs` (9 wiremock integration tests: create, function-tool round-trip, streaming happy/unknown/failed paths, retrieve, delete, cancel, `input_items` pagination).
- **Not ported** (v1 scope, noted for future work): `compact()`, `input_tokens.count()`, the `parse()` typed structured-output convenience wrapper, and the Responses-over-WebSocket connection class.

Verification: 156 tests passing (123 baseline + 23 new unit + 10 new integration), clippy clean (`-D warnings`).

## Unreleased — crate rename + final findings round (2026-07-04)

- **Renamed** crate `openai-rust` → **`openai-compat`** (the crates.io name `openai-rust` is taken); library path is now `openai_compat::`. All sources, tests, examples, and README updated.
- **Fixed** error `Display` parity with Python: non-JSON error bodies are used verbatim; JSON bodies keep the `Error code: {status} - {body}` prefix.
- **Added** typed logprobs: `ChatLogprobs`/`TokenLogprob`/`TopLogprob` (chat + chunks) and `CompletionLogprobs` (legacy) replace `serde_json::Value`.
- **Added** missing builders: images `style`/`user`, speech `instructions`, transcription `response_format`, legacy-completion setters (n/seed/echo/best_of/suffix/penalties/logprobs/user/stream_options), assistant-update setters (description/temperature/top_p/reasoning_effort/response_format).
- **Added** `ExpiresAfter` param type: `UploadCreateParams.expires_after` + `BatchCreateParams.output_expires_after`.
- **Changed** `Batches::list_all` accepts `Option<BatchListParams>` like other modules; fine-tuning `limit` params unified to `u32`.
- **Added** SSE decoder strips a leading U+FEFF BOM (per spec; ahead of the Python reference).
- **Added** `WebhookHeaders::from_lookup` for framework-agnostic header extraction; new `MissingHeaders` error variant.
- **Changed** cursor query building consolidated into shared `pagination::cursor_query` (removes 6 duplicated `to_query` bodies).
- **Verified** MSRV: `rust-version = "1.86"` (library `cargo check` clean on 1.86; capped by transitive icu deps. Dev-deps like wiremock need newer, which does not constrain downstream users).

Verification: 123 tests passing, clippy clean, `cargo publish --dry-run` green as `openai-compat v0.2.0`.

## Unreleased — full-crate review round (2026-07-04)

Four parallel code-reviewer passes over the whole crate (core pipeline, streaming/realtime/webhooks, v0.1 resources/docs, v0.2 resources/plan conformance). No critical findings; endpoint fidelity and plan conformance verified 100%. Fixes applied:

- **Added** missing high-use chat params: `reasoning_effort`, `parallel_tool_calls`, `logit_bias`, `metadata`, `store`, `service_tier`, `modalities`, `audio`, plus a `#[serde(flatten)] extra` map with `.param(key, value)` escape hatch on `ChatCompletionRequest`.
- **Fixed** `pub mod azure` leaking internal helpers into the public API (now private; Azure is configured only via `ClientBuilder`).
- **Fixed** transcription footgun: `transcriptions()` now rejects plain-text `response_format`s (`text`/`srt`/`vtt`) with a clear error; new `transcriptions_raw()` returns them as `String`.
- **Added** MIME inference from file extension in `FileUpload::from_path` (audio/json/image types), matching Python's mimetypes guess.
- **Added** SSE buffer cap (16 MiB) guarding against unbounded memory growth from a broken/malicious streaming server.
- **Fixed** webhook HMAC now computed over raw payload bytes (no lossy UTF-8 round-trip).
- **Fixed** realtime URL builder rejects base URLs containing a query/fragment instead of emitting a corrupt URL; documented `send()` cancel-safety and idle-ping caveats.
- **Changed** consolidated duplicated beta pagination (`assistants`/`vector_stores`) into shared `paginate_all_with_headers`; unified beta-header insertion order (caller override wins); added multi-page beta pagination regression test.
- **Changed** standardized request-param `metadata` to `HashMap<String, String>` (responses stay lenient `Value`).
- **Changed** Cargo.toml: reqwest `http2` feature enabled, `readme`/`rust-version = 1.75` metadata added, tokio features trimmed to `time`/`fs`/`net`, dropped inert `Config` re-export.

Verification: 121 tests passing, clippy clean, `cargo publish --dry-run` green.

## v0.2.0 — 2026-07-04

Full remaining `openai-python` core surface:

- **Added** multimodal message content parts (text / image_url / input_audio) via `MessageContent`/`ContentPart`.
- **Added** batches resource (create/retrieve/cancel/list with cursor pagination).
- **Added** resumable uploads resource (create/add part/complete/cancel, multipart).
- **Added** fine-tuning jobs resource (create/retrieve/list/cancel/pause/resume/events/checkpoints) with typed `method` (supervised/dpo/reinforcement) and shared `AutoOr<T>` for `"auto"|number` hyperparameters.
- **Added** vector stores resource (stores CRUD/list/search, files, file batches) with automatic `OpenAI-Beta: assistants=v2` header.
- **Added** assistants beta v2 (assistants, threads, messages, runs, run steps; `submit_tool_outputs`). Streaming runs not modeled.
- **Added** webhook signature verification (`webhooks::Webhooks`): HMAC-SHA256, `whsec_` secrets, constant-time compare, 300s timestamp tolerance.
- **Added** Azure OpenAI mode on the client builder (`.azure(endpoint, api_version)`, `.azure_deployment()`, `.azure_ad_token()`): `api-version` query on every request, `api-key`/Entra ID auth, deployment path derivation from request `model`.
- **Added** realtime WebSocket sessions (`realtime::connect`, `Client::connect_realtime`) over tokio-tungstenite/rustls with JSON events + typed event constructors.
- **Changed** license metadata from MIT to Apache-2.0 (matching the LICENSE file).
- **Changed** crate version to 0.2.0.

Post-review hardening (code review 2026-07-04, all findings addressed):

- **Fixed** Azure routing with a pinned deployment: non-deployment endpoints (`/models`, `/files`, ...) no longer receive a `/deployments/{dep}` segment (base URL is always `{endpoint}/openai`; the segment is added per request only for deployments endpoints, matching `azure.py::_prepare_url`).
- **Fixed** `ClientBuilder`/`AzureAuth` `Debug` output leaking `api_key`/`azure_ad_token` — now redacted.
- **Changed** `Client::connect_realtime` to reject Azure-configured clients explicitly (Azure realtime is out of scope; use `realtime::connect` with explicit options).

Verification: 120 tests passing (unit + wiremock integration + realtime local WS server + doctests), clippy clean.

## v0.1.0 — 2026-07-04

Initial port of the `openai-python` SDK core:

- Async client (reqwest/rustls + tokio) with builder, env fallbacks (`OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_ORG_ID`, `OPENAI_PROJECT_ID`), configurable base_url for any OpenAI-compatible provider.
- Retry engine mirroring `_base_client.py` (408/409/429/5xx + connection errors, 0.5s→8s backoff with jitter, `Retry-After`/`retry-after-ms`/`x-should-retry`).
- Typed error hierarchy with parsed error bodies and `x-request-id`.
- Hand-rolled SSE decoder + `EventStream<T>` (`futures::Stream`).
- Resources: chat completions (+streaming, tools, response_format), embeddings, models, moderations, legacy completions (+streaming), images, files (multipart), audio (speech/transcriptions).
- Cursor pagination (`List<T>`, `paginate_all`) and generic get/post/delete escape hatch.
- Post-review hardening: `#[non_exhaustive]` response structs, truncated-stream termination fix, per-read (httpx-style) timeout semantics, pagination cursor guard.
