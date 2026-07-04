---
title: "openai-rust v0.2 — Remaining Resource Surface"
description: "Port batches, uploads, fine-tuning, vector stores, assistants, webhooks, azure, realtime onto the v0.1 client."
status: pending
priority: P2
effort: 34h
branch: master
tags: [rust, openai, api-bindings, port]
created: 2026-07-04
---

# openai-rust v0.2 — Remaining Resource Surface

Port the remaining `openai-python` resource surface onto the existing v0.1 core
(Client/request/retry/pagination/streaming). Phases 02–07 are **independently
implementable in parallel** with strict file ownership: each owns ONLY its own
new files and must NOT edit shared files (`Cargo.toml`, `lib.rs`, `src/*/mod.rs`,
`client.rs`, `config.rs`, `request.rs`). All wiring is done by phase-08 (main agent).

## Compile note for parallel phases
Your new module cannot fully compile until phase-08 registers it and adds deps.
To self-verify, temporarily add your `pub mod x;` line and dep to `Cargo.toml`
locally, but **revert before handoff** — those edits belong to phase-08. Code
against the accessor signatures listed in `phase-08-wiring-tests-readme.md`.

## Phases

| # | Phase | Parallel group | Owner | New files | Status |
|---|-------|---------------|-------|-----------|--------|
| 01 | Chat multimodal content parts | A (serial, first) | main agent | edits `types/chat.rs` | in progress by main agent |
| 02 | Batches + Resumable uploads | B | subagent | `types/batches.rs`,`types/uploads.rs`,`resources/batches.rs`,`resources/uploads.rs`,`tests/batches-uploads-tests.rs` | pending |
| 03 | Fine-tuning jobs | B | subagent | `types/fine_tuning.rs`,`resources/fine_tuning.rs`,`tests/fine-tuning-tests.rs` | pending |
| 04 | Vector stores | B | subagent | `types/vector_stores.rs`,`resources/vector_stores.rs`,`tests/vector-stores-tests.rs` | pending |
| 05 | Assistants (beta v2) | B | subagent | `types/assistants.rs`,`resources/assistants.rs`,`tests/assistants-tests.rs` | pending |
| 06 | Webhooks + Azure client | B | subagent | `src/webhooks.rs`,`src/azure.rs`,`tests/webhooks-tests.rs` | pending |
| 07 | Realtime WebSockets | B | subagent | `src/realtime.rs`,`tests/realtime-tests.rs` | pending |
| 08 | Wiring + tests + README + review | C (serial, last) | main agent | all shared files | pending |

Group A runs first (main agent). Group B (02–07) runs fully in parallel. Group C
(main agent) merges: adds deps, registers modules, adds accessors, wires Azure into
`config.rs`/`request.rs`, updates README, runs `cargo build`/`clippy`/tests, code review.

## Key dependencies (added by phase-08)
- `hmac = "0.12"`, `sha2 = "0.10"`, `base64 = "0.22"`, `subtle = "2"` (constant-time cmp) — phase-06
- `tokio-tungstenite = { version = "0.24", default-features = false, features = ["rustls-tls-native-roots"] }` — phase-07

## Extracted constants (see phase files for detail)
- Webhook: HMAC-SHA256 over `"{id}.{timestamp}.{payload}"`, base64; headers `webhook-id`/`webhook-timestamp`/`webhook-signature` (space-sep `v1,<b64>`); `whsec_` prefix → base64-decode secret; tolerance 300s.
- Azure: base_url `{endpoint}/openai[/deployments/{dep}]`; `api-version` query on every request; auth `api-key: <key>` or `Authorization: Bearer <ad_token>`; 8 deployments endpoints.
- Beta headers: `OpenAI-Beta: assistants=v2`; `OpenAI-Beta: realtime=v1`.
- Realtime: `wss://.../realtime?model=<model>`; headers `Authorization: Bearer` + `OpenAI-Beta: realtime=v1`.
