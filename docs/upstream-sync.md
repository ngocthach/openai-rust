# Upstream Sync — tracking openai-python

This crate is a port of [openai-python](https://github.com/openai/openai-python).
This document records the exact upstream baseline the port matches, the
places where the port intentionally differs, and the process for absorbing
upstream changes.

## Current baseline

| | |
|---|---|
| openai-python version | **v3.40.0** |
| commit | `77fd116d2326bc828bb23d4fd7960142972293a4` |
| ported | 2026-10-28, openai-compat v0.4.0; parity fixes in v0.5.0 (2026-10-29) |

Previous baseline: v2.44.0 (`6d9262d5c666a1e4d47f63178db907ba3087ac5d`),
ported 2026-07-04 (v0.2.0); Responses API added 2026-07-05 (v0.3.0).

**Update this table every time a sync lands.**

## What is ported from where

| Rust module | Python source | What to watch |
|---|---|---|
| `src/retry.rs`, `src/request.rs` | `_base_client.py`, `_constants.py`, `_utils/_path.py` | retry statuses, backoff constants, new headers, path-parameter encoding |
| `src/config.rs`, `src/client.rs` | `_client.py`, `_data_residency.py` | env vars, default headers, new client options, data residency |
| `src/error.rs` | `_exceptions.py` | new error classes / body fields |
| `src/streaming.rs` | `_streaming.py` | SSE parsing changes, new sentinel handling |
| `src/pagination.rs` | `pagination.py` | new page shapes (e.g. `next` cursor variants) |
| `src/types/*`, `src/resources/*` | `types/`, `resources/`, **`api.md`** | new endpoints, new params, changed fields |
| `src/types/responses/*`, `src/resources/responses.rs` | `types/responses/`, `resources/responses/` | new stream event types, new tool types, `compact`, `input_tokens.count`; `parse` not ported |
| `src/resources/conversations.rs`, `src/types/conversations.rs` | `resources/conversations/`, `types/conversations/` | conversation and item endpoints, list envelope |
| `src/resources/chat.rs` (stored completions) | `resources/chat/completions/` | retrieve/update/list/delete/messages, `metadata[key]` filters |
| `src/webhooks.rs` | `resources/webhooks/webhooks.py`, `lib/_webhooks.py` | signature scheme changes |
| `src/azure.rs` + config/request wiring | `lib/azure.py` | deployments endpoint list, auth, URL rules |
| `src/realtime.rs` | `resources/realtime/realtime.py` | URL, headers, event envelope |

## Not ported

Grouped by status. The "deferred" rows were left out of scope for this
release; the rest are retired upstream or not planned for the crate.

| Area | Items |
|---|---|
| Deferred: newer upstream surfaces | Webhook endpoint management, audio voices/consents, decisions, safety, content provenance, ChatKit |
| Deferred: beta, admin, and cloud-provider surfaces | Beta agents/vaults, beta responses, live, admin, realtime translation, evals, containers, skills, Bedrock/Vertex/X.509 auth, realtime REST `client_secrets`/`calls`, fine-tuning checkpoint permissions/graders |
| Retired upstream | Videos (shut down 2026-09-24), image variations (endpoint no longer accepts requests) |
| Responses | `parse()` typed wrapper, WebSocket mode |
| Other typed fields | Reachable through `Response.extra` and the `extra` maps on Responses requests |
| Azure | `azure_ad_token_provider` (per-request token refresh); a static `azure_ad_token` must be rebuilt into a new client before it expires; `AZURE_OPENAI_ENDPOINT` env fallback |
| Other | Assistants streaming runs, fully-typed realtime events (events are `serde_json::Value` with typed constructors), resumable-upload helpers beyond the REST calls, sync client |

## Deliberate divergences

Behavior that differs from openai-python on purpose.

| Area | Divergence |
|---|---|
| Error message | Non-JSON error bodies get a truncated snippet appended to the message. Upstream drops it. |
| Error body | `ApiError.body` keeps the raw body, capped at 64 KiB. |
| Retry-After | HTTP-date form is unsupported and falls back to backoff. Upstream parses it. |
| Redirects | All clients refuse cross-origin (scheme, host, port) redirects with `OpenAIError::Redirect`; not retried. Upstream Azure strips `api-key` and follows; non-Azure follows. |
| Azure deployment | A pinned deployment is applied only to requests with a body, so body-less GETs (e.g. listing stored chat completions) are not routed to the deployment. Upstream applies it to every request via `base_url`. |
| Path parameters | Empty and dot-segment (`.`, `..`) ids are rejected client-side before any request is sent. Upstream rejects at the resource layer. |
| Realtime | A base URL with a query or fragment is rejected. No `call_id`. Upstream supports query strings. |
| Streaming | 16 MiB SSE buffer cap and WebSocket size limits. A lenient unterminated trailing SSE event is accepted. No post-`[DONE]` drain. |
| Response status | An unrecognized `ResponseStatus` deserializes to `Unknown` and serializes back as `"unknown"`; the original string is not kept. |
| Input token count | `InputTokenCountRequest.conversation` takes an id string only; the object form goes through `extra`. |
| Enums | Request-only enums have no `Unknown` variant. |
| Pagination | `list_all` by `last_id` returns an error when a page reports `has_more` with a missing or empty `last_id`. Upstream stops and keeps the items fetched so far. |
| User-Agent | Sends `openai-compat/{version}` and no `x-stainless-*` platform headers. Upstream sends `OpenAI/Python {version}` plus platform headers. |

## Sync process

1. Run `./scripts/check-upstream.sh` (requires a local openai-python clone;
   set `OPENAI_PYTHON_DIR` if it is not at `../openai-python`). It fetches
   upstream, then prints: version delta, commits since the baseline, the
   `api.md` diff (the API surface in one file), and per-file diffs for the
   core modules table above.
2. Triage the `api.md` diff first — it captures new endpoints/params/types in
   one place. `CHANGELOG.md` entries explain intent.
3. Port changes module-by-module; mirror constants and behaviors exactly, add
   or update wiremock tests for anything behavioral.
4. Run `cargo test && cargo clippy --all-targets` until green.
5. Update the baseline table above, the divergence list, and
   `docs/project-changelog.md`; bump the crate version (semver: new
   fields/endpoints = minor, breaking = major while pre-1.0 means minor).
6. `cargo publish`.

## Automation options

- Subscribe to releases: GitHub → openai/openai-python → Watch → Custom →
  Releases. Release cadence is high (several per month); syncing once every
  1-2 months and batching changes is usually the right granularity — most
  releases only touch generated types for new models.
- Or schedule a recurring agent to run the check and summarize what needs
  porting (e.g. Claude Code `/schedule` a monthly run of
  `./scripts/check-upstream.sh` + triage).
