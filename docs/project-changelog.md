# Project Changelog

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
