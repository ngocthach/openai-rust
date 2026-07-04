# Phase 03 — SSE Streaming (`Stream<T>`)

## Context Links
- Reference: `openai-python/src/openai/_streaming.py:283-385` (`SSEDecoder`, chunk splitting, field parsing)
- Reference: `_streaming.py:55-110` (`__stream__`: `[DONE]` handling, mid-stream error detection)
- Reference: `_streaming.py:242-281` (`ServerSentEvent`)

## Overview
- **Priority:** P1
- **Status:** pending
- Hand-rolled SSE parser + a `futures::Stream<Item = Result<T, OpenAIError>>` adapter over reqwest's byte stream. Powers `chat.completions.create_stream` and legacy `completions` streaming.

## Key Insights
- Chunk boundary (`_streaming.py:305-315`): accumulate bytes; a full event ends on `\r\r`, `\n\n`, or `\r\n\r\n`. Trailing partial flushed at end.
- Line parsing (`_streaming.py:339-385`):
  - Empty line → dispatch accumulated event (`data` joined by `\n`), reset event/data/retry (keep last_event_id).
  - Line starting `:` → comment, ignore.
  - `fieldname, _, value = line.partition(":")`; if value starts with one space, strip it.
  - Fields: `event` (set), `data` (append), `id` (set unless contains `\0`), `retry` (int).
- Stream consumption (`_streaming.py:62-107`):
  - If `sse.data` startswith `[DONE]` → end stream.
  - Parse `data` as JSON. If object has `error` key → raise `APIError(message = error.message or "An error occurred during streaming")`.
  - Else deserialize into `T` and yield.
  - (Skip the `thread.` Assistants special-case — out of scope.)

## Requirements
**Functional**
- `SseDecoder` (byte-fed, yields `ServerSentEvent { event, data, id, retry }`).
- `EventStream<T>`: `impl futures::Stream<Item = Result<T, OpenAIError>>` wrapping `reqwest::Response::bytes_stream()`.
- Terminates on `[DONE]`; surfaces mid-stream `{"error": ...}` as `OpenAIError::Api`/`Stream`.

**Non-functional**
- Decoder must handle event split across multiple byte chunks (buffer state). Unit-testable by feeding byte slices.

## Architecture
```
src/streaming.rs   # SseDecoder (sync push-parser) + EventStream<T> (async Stream adapter)
```
- `SseDecoder::feed(&mut self, chunk: &[u8]) -> Vec<ServerSentEvent>` (or push into buffer, return completed events). Keep decoder independent of async for direct unit tests.
- `EventStream<T>`: holds `bytes_stream` + `SseDecoder` + a queue of decoded events; implements `Stream` via `poll_next` (or build with `async_stream`-free manual state, OR use `futures::stream::unfold`). **Decision:** use `futures::stream::try_unfold` over `(response_stream, decoder, pending_deque)` for readability without extra deps.

## Related Code Files
**Create:** `src/streaming.rs`
**Modify:** `src/lib.rs` (re-export `EventStream`), `src/client.rs`/`src/request.rs` (helper `stream::<T>(response)` constructing `EventStream`).

## Implementation Steps
1. `ServerSentEvent { event: Option<String>, data: String, id: Option<String>, retry: Option<u64> }`.
2. `SseDecoder` with buffer `Vec<u8>`, current `event/data:Vec<String>/id/retry`. Implement:
   - `feed(chunk)`: append to buffer; extract complete lines (split on `\n`, handle `\r`); for each line call `decode_line`; on blank line emit event. Handle the three terminators. Return `Vec<ServerSentEvent>`.
   - `decode_line`: mirror `_streaming.py:339-385` field logic.
3. `EventStream<T>` via `try_unfold`:
   - Pull next decoded event from pending deque; if empty, poll `bytes_stream` for next chunk, `decoder.feed`, refill deque; on stream end flush decoder tail.
   - For each event: if `data` starts with `[DONE]` → `Ok(None)` (end).
   - `serde_json::from_str::<Value>`; if `.get("error")` present → `Err(OpenAIError::Stream(message))` (message from `error.message` or default).
   - Else `serde_json::from_value::<T>` → `Ok(Some((item, state)))`.
   - Map reqwest byte-stream errors → `OpenAIError::Connection`.
4. `cargo build`.

## Todo List
- [ ] ServerSentEvent + SseDecoder buffer/state
- [ ] decode_line field parsing (event/data/id/retry, comment, space-strip)
- [ ] event dispatch on blank line + terminator handling + tail flush
- [ ] EventStream<T> via try_unfold over bytes_stream
- [ ] [DONE] termination + mid-stream error mapping
- [ ] cargo build clean

## Success Criteria
- Feeding `data: {...}\n\ndata: [DONE]\n\n` yields one deserialized item then ends.
- Event split across two `feed` calls is parsed correctly.
- `data: {"error": {"message": "boom"}}` → stream yields `Err` with "boom".

## Risk Assessment
- Backpressure/partial UTF-8 across chunk boundary: buffer at byte level, only decode complete lines. Mitigation: operate on bytes, `String::from_utf8_lossy` per line.

## Security Considerations
- None beyond error message hygiene.

## Next Steps
- Phase 04 wires `EventStream<ChatCompletionChunk>` to `create_stream`.
