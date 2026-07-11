# Development Roadmap

## Phase 1 — Core client (v0.1) — ✅ Complete (2026-07-04)

Client/builder/config, retries, SSE streaming, errors, pagination, and the
core resources: chat, embeddings, models, moderations, legacy completions,
images, files, audio. 45 tests, reviewed.

## Phase 2 — Full surface parity (v0.2) — ✅ Complete (2026-07-04)

Multimodal content parts, batches, resumable uploads, fine-tuning, vector
stores, assistants beta v2, webhook verification, Azure OpenAI mode, realtime
WebSockets. 118 tests, clippy clean.

## Phase 3 — Responses API (v0.3) — ✅ Complete (2026-07-05)

`/v1/responses` resource: create/create_stream, retrieve/retrieve_stream
(background-mode resumable streaming), delete, cancel, `input_items` cursor
pagination. Typed input/tool unions, `Response`/`OutputItem`, ~12 typed stream
events + `Unknown` catch-all. See `docs/upstream-sync.md` for what's
intentionally not ported (`compact()`, `input_tokens.count()`, `parse()`,
Responses-over-WebSocket).

## Future (unscheduled)

- Assistants streaming runs (SSE `AssistantStreamEvent`).
- Fully-typed realtime event enums (currently `serde_json::Value` + typed constructors).
- Vector store / run poll helpers (`create_and_poll`).
- Typed webhook event union.
