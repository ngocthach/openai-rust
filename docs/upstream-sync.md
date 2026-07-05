# Upstream Sync — tracking openai-python

This crate is a port of [openai-python](https://github.com/openai/openai-python).
This document records the exact upstream baseline the port matches and the
process for absorbing upstream changes.

## Current baseline

| | |
|---|---|
| openai-python version | **v2.44.0** |
| commit | `6d9262d5c666a1e4d47f63178db907ba3087ac5d` (2026-06-24) |
| ported | 2026-07-04, openai-compat v0.2.0 (Responses API added 2026-07-05, v0.3.0) |

**Update this table every time a sync lands.**

## What is ported from where

| Rust module | Python source | What to watch |
|---|---|---|
| `src/retry.rs`, `src/request.rs` | `_base_client.py`, `_constants.py` | retry statuses, backoff constants, new headers |
| `src/config.rs`, `src/client.rs` | `_client.py` | env vars, default headers, new client options |
| `src/error.rs` | `_exceptions.py` | new error classes / body fields |
| `src/streaming.rs` | `_streaming.py` | SSE parsing changes, new sentinel handling |
| `src/pagination.rs` | `pagination.py` | new page shapes (e.g. `next` cursor variants) |
| `src/types/*`, `src/resources/*` | `types/`, `resources/`, **`api.md`** | new endpoints, new params, changed fields |
| `src/types/responses/*`, `src/resources/responses.rs` | `types/responses/`, `resources/responses/` | new stream event types, new tool types, `compact`/`input_tokens.count`/`parse` (not yet ported) |
| `src/webhooks.rs` | `resources/webhooks.py` | signature scheme changes |
| `src/azure.rs` + config/request wiring | `lib/azure.py` | deployments endpoint list, auth, URL rules |
| `src/realtime.rs` | `resources/realtime/realtime.py` | URL, headers, event envelope |

Not ported (intentional): Responses API `compact()`/`input_tokens.count()`/
`parse()`/WebSocket connection (core create/retrieve/delete/cancel/streaming
IS ported, see table above), evals, containers, videos, skills, assistants
streaming runs, fully-typed realtime events, resumable-upload
helpers beyond the REST calls, sync client.

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
5. Update the baseline table above + `docs/project-changelog.md`; bump the
   crate version (semver: new fields/endpoints = minor, breaking = major
   while pre-1.0 means minor).
6. `cargo publish`.

## Automation options

- Subscribe to releases: GitHub → openai/openai-python → Watch → Custom →
  Releases. Release cadence is high (several per month); syncing once every
  1-2 months and batching changes is usually the right granularity — most
  releases only touch generated types for new models.
- Or schedule a recurring agent to run the check and summarize what needs
  porting (e.g. Claude Code `/schedule` a monthly run of
  `./scripts/check-upstream.sh` + triage).
