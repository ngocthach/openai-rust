# Phase 06 — Tests, Examples, Docs/README

## Context Links
- Reference behaviors validated against phases 01-05.
- `wiremock` docs for HTTP mocking (`docs-seeker`/context7 if needed).

## Overview
- **Priority:** P2
- **Status:** pending
- Unit + integration tests (mocked HTTP), runnable examples, crate/README docs. No CI required.

## Key Insights
- Pure functions from `retry.rs` unit-test exact backoff without network.
- `wiremock` spins a local server; point `Client` at its `base_url` to test full request/retry/error/pagination/streaming paths.
- SSE decoder unit-tested by feeding byte slices directly (no server).

## Requirements
**Functional**
- Unit tests: retry timeout math, should_retry, retry-after parsing, error status→kind mapping, error body parse, SSE decoder (multi-chunk, [DONE], mid-stream error), pagination cursor advance.
- Integration (wiremock): retry on 429 then 200; non-retry on 400; chat create happy path; chat stream (mock SSE response); pagination two-page walk; multipart upload accepted.
- Doc examples: crate-level + `create` + `create_stream` (as `no_run` doctests).

## Architecture
```
tests/
├── retry.rs           # unit-ish via pub(crate) or expose test helpers
├── errors.rs
├── sse.rs
├── pagination.rs
├── chat_create.rs     # wiremock
└── chat_stream.rs     # wiremock SSE
examples/
├── chat.rs            # basic create
└── chat_stream.rs     # streaming print deltas
README.md              # usage, install, feature table, provider base_url note
```
Note: to unit-test `retry.rs`/`streaming.rs` internals, add `#[cfg(test)] mod tests` inside those files (pure fns), and use `tests/` dir for wiremock integration.

## Related Code Files
**Create:** `tests/*.rs`, `examples/chat.rs`, `examples/chat_stream.rs`, update `README.md`
**Modify:** source files to add in-module `#[cfg(test)]` unit tests; ensure needed items are `pub(crate)` or test-visible.

## Implementation Steps
1. In `retry.rs`: `#[cfg(test)]` tests asserting `calculate_retry_timeout(0,2,None,0.0)` etc. match `0.5*2^n * (1-0.25*rand)` bounds; retry-after in-range/out-of-range; should_retry per status + header override.
2. In `error.rs`: tests for `from_response` mapping (400/401/403/404/409/422/429/500) + body parse + request_id extraction.
3. In `streaming.rs`: decoder tests (split events, comments, `data:` space strip, `[DONE]`, error event).
4. `tests/chat_create.rs`: wiremock mount `POST /chat/completions` → JSON fixture; assert parsed `ChatCompletion`. Add a 429-then-200 retry scenario asserting 2 calls.
5. `tests/chat_stream.rs`: wiremock respond with `text/event-stream` body of `data: {chunk}\n\n... data: [DONE]\n\n`; collect chunks.
6. `tests/pagination.rs`: two mocked pages (`has_more:true` + `after` cursor, then `has_more:false`); assert combined items.
7. `examples/chat.rs` + `examples/chat_stream.rs`: read `OPENAI_API_KEY`, print result (compile-checked; run manual).
8. README: quickstart, `base_url` override for OpenAI-compatible providers, supported endpoints table, error/retry/stream notes.
9. `cargo test` all green; `cargo build --examples`; `cargo doc --no-deps`.

## Todo List
- [ ] retry.rs unit tests
- [ ] error.rs mapping/body/request_id tests
- [ ] streaming.rs decoder tests
- [ ] tests/chat_create.rs (happy + retry) wiremock
- [ ] tests/chat_stream.rs wiremock SSE
- [ ] tests/pagination.rs two-page walk
- [ ] examples/chat.rs + chat_stream.rs
- [ ] README.md rewrite
- [ ] cargo test / build --examples / doc clean

## Success Criteria
- `cargo test` passes; retry/error/SSE/pagination covered.
- Examples compile; README documents install + provider base_url + endpoint coverage.
- No failing/ignored tests to force green (per project rules).

## Risk Assessment
- wiremock streaming body: ensure response sets `Content-Type: text/event-stream` and full body delivered; if reqwest buffers, decoder still parses. Mitigation: send complete SSE body in one response.

## Security Considerations
- Examples must read key from env, never hardcode.

## Next Steps
- Hand off to `code-reviewer`; update `docs/` changelog/roadmap if present.
