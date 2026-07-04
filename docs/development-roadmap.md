# Development Roadmap

## Phase 1 — Core client (v0.1) — ✅ Complete (2026-07-04)

Client/builder/config, retries, SSE streaming, errors, pagination, and the
core resources: chat, embeddings, models, moderations, legacy completions,
images, files, audio. 45 tests, reviewed.

## Phase 2 — Full surface parity (v0.2) — ✅ Complete (2026-07-04)

Multimodal content parts, batches, resumable uploads, fine-tuning, vector
stores, assistants beta v2, webhook verification, Azure OpenAI mode, realtime
WebSockets. 118 tests, clippy clean.

## Future (unscheduled)

- Assistants streaming runs (SSE `AssistantStreamEvent`).
- Fully-typed realtime event enums (currently `serde_json::Value` + typed constructors).
- Vector store / run poll helpers (`create_and_poll`).
- Typed webhook event union.
- Responses API (`/responses`) — newer OpenAI primary API.
- crates.io release automation (CI, docs.rs metadata).
