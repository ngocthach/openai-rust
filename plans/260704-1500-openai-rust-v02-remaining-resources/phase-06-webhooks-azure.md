# Phase 06 — Webhooks + Azure Client

**Owner:** subagent. **Parallel group:** B.
**File ownership (create ONLY these):**
- `src/webhooks.rs` (standalone verification logic — no Config dependency)
- `src/azure.rs` (standalone Azure config types + pure path/auth helpers)
- `tests/webhooks-tests.rs`

Do NOT edit `config.rs`, `request.rs`, `client.rs`, `lib.rs`, `Cargo.toml`.
Phase-08 adds deps and calls your `azure.rs` helpers from `config.rs`/`request.rs`
and adds `client.webhooks()`. Design your public functions so phase-08 only wires.

## Context Links
- Python: `resources/webhooks/webhooks.py` (verify_signature L44-115, unwrap L23-42),
  `lib/azure.py` (`_deployments_endpoints` L21-32, base_url L247-249, `_build_request`
  L67-79, `_prepare_url` L81-97, `_auth_headers` L355-362, default_query api-version L232-235).
- `_exceptions.py::InvalidWebhookSignatureError`.

## New dependencies (phase-08 adds to Cargo.toml)
`hmac = "0.12"`, `sha2 = "0.10"`, `base64 = "0.22"`, `subtle = "2"` (constant-time compare).

## ============ WEBHOOKS ============

### Algorithm (EXACT — record verbatim)
1. Required headers (case-insensitive): `webhook-signature`, `webhook-timestamp`, `webhook-id`.
2. Timestamp: parse `webhook-timestamp` as i64 seconds. Reject if not integer
   (`InvalidWebhookSignature("Invalid webhook timestamp format")`).
   - `now - ts > tolerance` → "Webhook timestamp is too old".
   - `ts > now + tolerance` → "Webhook timestamp is too new".
   - **tolerance default = 300 seconds.**
3. Signatures: split `webhook-signature` on ASCII whitespace; for each part, strip a
   leading `"v1,"` prefix (keep remainder); parts without prefix taken as-is. Collect all.
4. Secret decode: if secret starts with `"whsec_"`, base64-**decode** `secret[6..]` →
   key bytes; else use `secret.as_bytes()`.
5. Signed payload string = `format!("{webhook_id}.{timestamp}.{body}")` where `body`
   is the raw payload (bytes → utf-8 str). `timestamp` is the ORIGINAL header string
   (not re-serialized).
6. `expected = base64_standard( HMAC_SHA256(key, signed_payload) )`.
7. Accept if `expected` equals ANY provided signature via **constant-time** compare
   (`subtle::ConstantTimeEq` on equal-length byte slices; guard length first).
   Else `InvalidWebhookSignature("The given webhook signature does not match the expected signature")`.

### Rust API (standalone — no Config needed)
```rust
pub struct Webhooks; // or free fns
impl Webhooks {
  pub fn verify_signature(payload: &[u8], headers: &HeaderMap, secret: &str,
                          tolerance_secs: i64 /*default 300*/) -> Result<(), OpenAIError>;
  pub fn unwrap(payload: &[u8], headers: &HeaderMap, secret: &str)
      -> Result<WebhookEvent, OpenAIError>; // verifies then parses JSON
}
```
- `WebhookEvent` typed envelope: `{ id:String, type:String (rename "type"),
  created_at:i64, data:serde_json::Value, object?:String }` (keep event body as `Value`;
  full typed union of event types OUT of scope — document).
- Add `OpenAIError::InvalidWebhookSignature(String)` variant. **This edits `error.rs`
  (shared).** RESOLUTION: define a local error in `webhooks.rs` OR return
  `OpenAIError::Api`-style; simplest is to reuse an existing variant with a message.
  Phase-08 adds the dedicated `InvalidWebhookSignature` variant to `error.rs`; until
  then use `OpenAIError::Config` as a placeholder and mark `// TODO(phase-08): dedicated variant`.
- Accept headers as `reqwest::header::HeaderMap` (already a dep) for ergonomic lookup;
  provide a `&[(String,String)]` overload if simpler.

## ============ AZURE ============

### Constants (EXACT)
- `DEPLOYMENTS_ENDPOINTS` (8): `/completions`, `/chat/completions`, `/embeddings`,
  `/audio/transcriptions`, `/audio/translations`, `/audio/speech`,
  `/images/generations`, `/images/edits`.
- base_url: with deployment → `{endpoint.trim_end('/')}/openai/deployments/{deployment}`;
  without → `{endpoint.trim_end('/')}/openai`.
- `api-version` query param appended to **every** request.
- Auth: `azure_ad_token` present → header `Authorization: Bearer {token}`; else API key →
  header `api-key: {key}`.
- Realtime (see phase-07): query adds `api-version` + `deployment` (deployment or model).

### Path rule (from `_build_request`/`_prepare_url`)
- If a deployment IS configured, base_url already contains `/deployments/{dep}`; append
  request path directly. (Assistants NOT supported with deployment — document.)
- If NO deployment configured and the request path ∈ `DEPLOYMENTS_ENDPOINTS` and body is
  JSON with a `model` field: rewrite path to `/deployments/{model}{path}`.
- Otherwise append path to `{endpoint}/openai` unchanged.

### Rust API (pure, in `azure.rs`; phase-08 integrates)
```rust
pub enum AzureAuth { ApiKey(String), AdToken(String) }
pub struct AzureConfig { pub endpoint: String, pub api_version: String,
                         pub deployment: Option<String>, pub auth: AzureAuth }
impl AzureConfig {
  pub fn base_url(&self) -> String;                 // {endpoint}/openai[/deployments/{dep}]
  pub fn auth_header(&self) -> (String, String);    // ("api-key",k) | ("Authorization","Bearer t")
  pub fn rewrite_path(&self, path: &str, json_body: Option<&serde_json::Value>) -> String;
}
pub const DEPLOYMENTS_ENDPOINTS: &[&str] = &[ /* 8 above */ ];
```
Phase-08 adds to `config.rs`: `api_version`/`azure` fields + builder
`.azure(endpoint, api_version)`, `.azure_deployment(dep)`, `.azure_ad_token(token)`,
and a `default_query: Vec<(String,String)>` (holds `("api-version", v)`); and in
`request.rs` merges `default_query` into every request, calls `rewrite_path`, and picks
the auth header from `auth_header()` instead of the default Bearer.

## Related Code Files
- Create: `src/webhooks.rs`, `src/azure.rs`, `tests/webhooks-tests.rs`.
- Phase-08 edits: `error.rs` (new variant), `config.rs`, `request.rs`, `client.rs`,
  `lib.rs`, `Cargo.toml`.

## Implementation Steps
1. `webhooks.rs`: implement `verify_signature`/`unwrap` + `WebhookEvent` + unit tests
   (known secret/timestamp/payload → known signature; tamper → error; old/new ts; whsec_).
2. `azure.rs`: `AzureConfig`/`AzureAuth` + `base_url`/`auth_header`/`rewrite_path` +
   `DEPLOYMENTS_ENDPOINTS`; unit tests for path rewriting (deployment vs model-in-body).
3. `tests/webhooks-tests.rs`: end-to-end verify against a fixture computed by the same
   algorithm; assert timestamp-tolerance and tamper rejection.

## Todo List
- [ ] `verify_signature` (headers, tolerance, whsec_, v1 prefix, constant-time)
- [ ] `unwrap` + `WebhookEvent` envelope
- [ ] `AzureConfig` base_url/auth_header/rewrite_path + `DEPLOYMENTS_ENDPOINTS`
- [ ] webhook unit + integration tests; azure path-rewrite unit tests
- [ ] local `cargo test` (add deps locally, revert)

## Success Criteria
- Webhook: valid signature passes; wrong secret/tampered payload/expired ts fail with the
  exact messages; `whsec_` base64 secret handled; multiple space-separated `v1,` sigs.
- Azure: base_url + api-version + auth header + deployment/model path rewrite match `azure.py`.

## Risk Assessment
- Non-constant-time compare = timing attack → use `subtle`.
- Base64 alphabet: use STANDARD (not URL-safe) for both signature and `whsec_` decode.

## Security Considerations
- Constant-time signature comparison; never log secret or raw signature; treat api-key
  header as sensitive (`set_sensitive(true)` in phase-08 wiring).

## Next Steps
- Phase-08 integrates azure into config/request, adds error variant + `client.webhooks()`.
