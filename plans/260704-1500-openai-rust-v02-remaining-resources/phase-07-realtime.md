# Phase 07 — Realtime WebSockets

**Owner:** subagent. **Parallel group:** B.
**File ownership (create ONLY these):**
- `src/realtime.rs`
- `tests/realtime-tests.rs`

Do NOT edit `client.rs`, `lib.rs`, `Cargo.toml`. Phase-08 adds the dep and
`client.realtime()`. Design `connect` to take `base_url` + `api_key` as params so it
needs no Config access.

## Context Links
- Python: `resources/beta/realtime/realtime.py` (header `OpenAI-Beta: realtime=v1` L384,
  L567); newer `resources/realtime/realtime.py` (URL build `_prepare_url` L1190-1199,
  query/model L1167-1173, send L825-838, recv/parse_event L805-859).
- Azure realtime query (`lib/azure.py::_configure_realtime` L395-418): adds `api-version`
  + `deployment`.

## New dependency (phase-08 adds)
`tokio-tungstenite = { version = "0.24", default-features = false, features = ["rustls-tls-native-roots"] }`
(pairs with existing rustls stack). Pulls `tungstenite` for `Message`.

## Extracted constants (EXACT)
- URL: derive ws scheme from http base — `http`→`ws`, `https`→`wss`. Append `/realtime`
  to base path (strip trailing `/`). Add query `model=<model>` (+ any extra query).
  Default OpenAI: `wss://api.openai.com/v1/realtime?model=<model>`.
- Headers on connect: `Authorization: Bearer <api_key>` and `OpenAI-Beta: realtime=v1`.
- Event envelope: every JSON event has a `"type"` string; most carry `"event_id"`.

## Common event `type` strings (verbatim)
- Client→server: `session.update`, `input_audio_buffer.append`, `input_audio_buffer.commit`,
  `input_audio_buffer.clear`, `conversation.item.create`, `conversation.item.delete`,
  `conversation.item.truncate`, `response.create`, `response.cancel`.
- Server→client (observed): `session.created`, `session.updated`, `response.created`,
  `response.done`, `conversation.item.created`, `input_audio_buffer.committed`, `error`.
- Full 86-event typed surface OUT of scope — use `serde_json::Value` with the envelope.

## Overview
- **Priority:** P3. **Status:** pending.
- Thin async session: send/receive JSON events over WSS, plus typed constructors for the
  3 common client events.

## Architecture
```rust
pub struct RealtimeSession { ws: WebSocketStream<...> }
impl RealtimeSession {
  // phase-08 calls this from client.realtime().connect(model)
  pub async fn connect(base_url:&str, api_key:&str, model:&str,
                       extra_query:&[(String,String)]) -> Result<Self, OpenAIError>;
  pub async fn send_event(&mut self, event: serde_json::Value) -> Result<(), OpenAIError>;
  pub async fn next_event(&mut self) -> Option<Result<serde_json::Value, OpenAIError>>;
  pub async fn close(&mut self) -> Result<(), OpenAIError>;
}
// Typed helper constructors returning serde_json::Value:
pub fn session_update(session: serde_json::Value) -> serde_json::Value; // {"type":"session.update","session":..}
pub fn input_audio_buffer_append(audio_b64: &str) -> serde_json::Value;  // {"type":"input_audio_buffer.append","audio":..}
pub fn response_create(response: Option<serde_json::Value>) -> serde_json::Value; // {"type":"response.create",..}
```
- Build request with `tokio_tungstenite::connect_async` using a
  `http::Request` carrying the two headers (tungstenite `client_async` /
  `IntoClientRequest`). Text frames only; `send_event` = `Message::Text(to_string)`;
  `next_event` decodes `Message::Text`/`Binary` → `serde_json::from_slice`, skips
  Ping/Pong, ends on Close.

## Related Code Files
- Create: `src/realtime.rs`, `tests/realtime-tests.rs`.
- Phase-08 edits: `Cargo.toml` (dep), `client.rs` (`Realtime`/`connect` accessor),
  `lib.rs` (`pub mod realtime;` + re-export `RealtimeSession`).

## Accessor signatures (phase-08 will add)
```rust
impl Client { pub fn realtime(&self) -> Realtime; }
// Realtime::connect(&self, model:&str) -> Result<RealtimeSession, OpenAIError>
//   uses client base_url (ws-ified) + api_key + OpenAI-Beta: realtime=v1
```

## Implementation Steps
1. `realtime.rs`: URL builder (http→ws, https→wss, append `/realtime`, add `model`),
   `connect` with the two headers, `send_event`/`next_event`/`close`, 3 helper ctors.
2. Frame encode/decode helpers isolated as pure fns for unit testing.
3. `tests/realtime-tests.rs`: unit-level — (a) URL builder cases (http/https, trailing
   slash, extra query), (b) helper ctor JSON shape, (c) frame encode/decode round-trip.
   A live WS server test is optional; prefer unit-level per scope.

## Todo List
- [ ] URL builder (scheme swap, path, model query) + unit tests
- [ ] `connect` with Authorization + `OpenAI-Beta: realtime=v1`
- [ ] `send_event`/`next_event`/`close`
- [ ] 3 typed helper constructors
- [ ] frame encode/decode round-trip tests
- [ ] local `cargo test` (add dep locally, revert)

## Success Criteria
- URL builder produces `wss://.../realtime?model=...`; connect sets both headers;
  send/next round-trip a JSON event over an in-test tungstenite stream (or mock).

## Risk Assessment
- TLS feature mismatch — must use a rustls feature to match existing stack; verify
  `tokio-tungstenite` rustls feature name at impl time.
- `IntoClientRequest` header injection API varies by version — pin 0.24 and adapt.

## Security Considerations
- Bearer token in WS upgrade header only; do not log frames containing audio/keys.

## Next Steps
- Phase-08 adds dep + `client.realtime()`. Document streaming-run and full event typing
  as future work in README.
