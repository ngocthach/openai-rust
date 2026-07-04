---
title: "openai-rust: Async Rust Client for OpenAI-Compatible APIs"
description: "Idiomatic async Rust port of the openai-python SDK core + v0.1 resource surface."
status: complete (2026-07-04) — reviewed, action items resolved
priority: P2
effort: 24h
branch: master
tags: [rust, sdk, openai, async, http-client]
created: 2026-07-04
---

# openai-rust Client Library

Idiomatic async Rust client for OpenAI-compatible LLM providers (configurable `base_url`).
Ports the core infrastructure + v0.1 resource surface of `openai-python`. Async-only
(reqwest + tokio), YAGNI/KISS/DRY. Reference SDK: `/Users/ngocthach/myGit/openai-python/`.

## Design Decisions
- Single crate `openai-rust`, module tree (client/config/error/streaming/pagination/types/resources). Each file < 800 lines.
- Async-only. `reqwest` (rustls-tls), `tokio`, `serde`/`serde_json`, `futures`, `thiserror`, `bytes`, `rand`.
- Hand-rolled minimal SSE parser (no `eventsource-stream` dep) mirroring `_streaming.py::SSEDecoder`.
- Builder pattern for `Client`; per-resource accessors (`client.chat().completions()`, etc.).
- `#[serde(skip_serializing_if = "Option::is_none")]` for optional request params; `#[non_exhaustive]` response structs; `#[serde(other)]` fallback for enums to stay forward-compatible with OpenAI-compatible providers.
- Escape hatch: generic `client.get::<T>()` / `client.post::<B,T>()` returning typed or `serde_json::Value`.

## Phases
| # | Phase | Status | Effort |
|---|-------|--------|--------|
| 01 | Scaffolding + config + client core + error hierarchy | complete | 4h |
| 02 | Request layer + retry logic | complete | 4h |
| 03 | SSE streaming (`Stream<T>`) | complete | 3h |
| 04 | chat.completions + types (create + create_stream) | complete | 5h |
| 05 | Remaining resources + pagination | complete | 5h |
| 06 | Tests + examples + docs/README | complete | 3h |

## Extracted Constants (from `_constants.py`)
- `DEFAULT_TIMEOUT` = 600s (connect 5s); `DEFAULT_MAX_RETRIES` = 2
- `INITIAL_RETRY_DELAY` = 0.5s; `MAX_RETRY_DELAY` = 8.0s
- Backoff: `min(0.5 * 2^nb_retries, 8.0)`; jitter `1 - 0.25*rand()` (0.75x..1.0x)
- `retry-after` cap: use header directly only if `0 < retry_after <= 60`
- Retryable: `x-should-retry` override, else 408/409/429/>=500 + connection/timeout errors
- Default `base_url` = `https://api.openai.com/v1`

## Dependencies
See phase-01 for exact `Cargo.toml` versions.

## Out of Scope
realtime/ws, assistants/beta, batches, fine-tuning, vector stores, resumable uploads,
webhooks, evals, containers, videos, azure client, structured-output runtime helper, sync client.

## Review Findings (2026-07-04) — Next Steps
Build/clippy/tests all green (30 unit + 14 integration + 1 doctest). No CRITICAL/HIGH security issues; API-key redaction + `set_sensitive` verified. Action items resolved 2026-07-04:
- [x] **HIGH (API):** `#[non_exhaustive]` added to all 22 response-shaped structs (chat, common::Usage, embeddings, models, moderations, completions, images, files, audio, pagination::List).
- [x] **MEDIUM (correctness):** `EventStream::poll_next` no longer re-polls the exhausted byte stream — `bytes_exhausted` flag added + regression test `truncated_stream_without_done_terminates_cleanly`.
- [x] **MEDIUM (fidelity):** client now uses `read_timeout` (per read op, httpx semantics) instead of a whole-request deadline, so long streams are not aborted; per-request total deadline still available via `RequestOptions::timeout`.
- [x] **MEDIUM (correctness):** `paginate_all` breaks when the `after` cursor does not advance (guards providers that ignore the cursor or omit `has_more`).
- [x] **LOW:** SSE `flush` strips a held-back trailing `\r` before processing the tail.
- Deferred (accepted): `SseDecoder::feed` O(n²) worst case on pathological buffers — irrelevant at typical SSE chunk sizes (KISS).

## Open Questions
- Response struct forward-compat strategy: capture unknown fields via `#[serde(flatten)] extra: Map` vs drop. Recommend drop for v0.1 (KISS); revisit if providers need passthrough.
