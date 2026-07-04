# Phase 08 — Wiring, Tests, README, Review

**Owner:** main agent. **Parallel group:** C (serial, LAST — after 01–07 merge).
**Owns ALL shared-file edits.** This phase makes the crate compile as a whole.

## Context Links
- Depends on merged output of phases 01–07.
- Shared files: `Cargo.toml`, `src/lib.rs`, `src/types/mod.rs`, `src/resources/mod.rs`,
  `src/client.rs`, `src/config.rs`, `src/request.rs`, `src/error.rs`, `README.md`.

## Overview
- **Priority:** P1 (integration). **Status:** pending.
- Add deps, register modules, add all accessors, wire Azure into config/request, add
  webhook error variant, update README, run full build/clippy/test, code review.

## 1. Cargo.toml — add dependencies
```toml
hmac = "0.12"
sha2 = "0.10"
base64 = "0.22"
subtle = "2"
tokio-tungstenite = { version = "0.24", default-features = false, features = ["rustls-tls-native-roots"] }
```
Bump crate `version = "0.2.0"`.

## 2. Module registration
- `src/lib.rs`: add `mod webhooks;`, `pub mod realtime;`, `mod azure;` (or `pub`),
  re-export `RealtimeSession`, `WebhookEvent`, azure builder types as needed.
- `src/types/mod.rs`: add `pub mod batches; pub mod uploads; pub mod fine_tuning;
  pub mod vector_stores; pub mod assistants;`.
- `src/resources/mod.rs`: add `pub mod batches; pub mod uploads; pub mod fine_tuning;
  pub mod vector_stores; pub mod assistants;`.

## 3. client.rs — accessors (exact signatures)
```rust
pub fn batches(&self) -> Batches
pub fn uploads(&self) -> Uploads
pub fn fine_tuning(&self) -> FineTuning
pub fn vector_stores(&self) -> VectorStores
pub fn beta(&self) -> Beta                 // -> assistants()/threads()
pub fn webhooks(&self) -> Webhooks         // or free fn module
pub fn realtime(&self) -> Realtime         // Realtime::connect(model)
```
Each `X::new(self.clone())`, mirroring existing accessors.

## 4. error.rs — webhook variant
Add `#[error("invalid webhook signature: {0}")] InvalidWebhookSignature(String)` to
`OpenAIError`; replace phase-06 placeholder uses.

## 5. Azure wiring (config.rs + request.rs)
- `config.rs`: add fields `default_query: Vec<(String,String)>` and
  `azure: Option<AzureConfig>` (from `azure.rs`). Builder methods:
  `.azure(endpoint, api_version)`, `.azure_deployment(dep)`, `.azure_ad_token(token)`.
  On build: if azure, set `base_url = azure.base_url()`, push `("api-version", api_version)`
  into `default_query`. Env fallbacks (optional): `AZURE_OPENAI_ENDPOINT`,
  `OPENAI_API_VERSION`, `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_AD_TOKEN`.
- `request.rs` `build_request`: merge `config.default_query` into every request's query;
  if `config.azure` is set, call `azure.rewrite_path(path, json_body)` and use
  `azure.auth_header()` (set `api-key` sensitive) INSTEAD of the default `Authorization:
  Bearer`. Non-azure path unchanged.
- `request_headers`: skip the default Bearer when azure provides its own auth header.

## 6. README.md — v0.2 section
Add a "Resources" section: batches, uploads, fine-tuning, vector stores, assistants
(beta), webhooks, azure, realtime — one short example each. Document OUT-of-scope:
streaming runs, full realtime event typing, vector-store poll helpers.

## 7. Integration tests
- Ensure each phase's `tests/*.rs` compiles and passes against wiremock.
- Add `tests/azure-tests.rs`: wiremock asserting `api-version` query, `api-key` header,
  and `/openai/deployments/{dep}` path.

## 8. Verification gates (run in order, fix before proceeding)
1. `cargo build` clean.
2. `cargo clippy --all-targets` — no errors.
3. `cargo test` — all pass (delegate to `tester` agent).
4. Delegate to `code-reviewer` agent; address findings.
5. Update `docs/project-changelog.md` + `docs/development-roadmap.md` (v0.2 entry).

## Todo List
- [ ] Add deps + bump version
- [ ] Register all modules (lib/types/resources mod.rs)
- [ ] Add all client accessors
- [ ] Add `InvalidWebhookSignature` error variant
- [ ] Azure config fields + builder + request.rs query/path/auth wiring
- [ ] README v0.2 resources section
- [ ] `tests/azure-tests.rs`
- [ ] build + clippy + test green
- [ ] code-reviewer pass
- [ ] changelog + roadmap update

## Success Criteria
- Whole crate builds; all resource accessors reachable; azure request shape matches
  `azure.py`; all integration tests pass; clippy clean; review addressed.

## Risk Assessment
- Merge conflicts across mod.rs — only phase-08 edits them, so none if phases obeyed ownership.
- Azure auth branch must not double-send `Authorization` + `api-key` — assert in tests.
- reqwest query merge order: `default_query` then per-request query (both applied).

## Security Considerations
- api-key / AD token headers marked sensitive; webhook constant-time compare retained;
  no secrets in README examples (use env vars).

## Next Steps
- Tag v0.2.0. Future: streaming runs (SSE AssistantStreamEvent), full realtime event enums,
  poll helpers, typed webhook event union.
