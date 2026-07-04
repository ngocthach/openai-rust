# Phase 04 — chat.completions + Types

## Context Links
- Reference: `openai-python/src/openai/resources/chat/completions/completions.py:250-295` (create signature, all params)
- Reference: `openai-python/src/openai/types/chat/chat_completion.py` (`Choice`, `ChatCompletion`, finish_reason, usage)
- Reference: `openai-python/src/openai/types/chat/chat_completion_chunk.py` (`ChoiceDelta`, `ChoiceDeltaToolCall`, chunk `Choice`)
- Reference: `openai-python/src/openai/types/chat/chat_completion_message.py`, `.../chat_completion_message_tool_call.py`, `.../completion_usage.py`
- Endpoint: `POST /chat/completions`

## Overview
- **Priority:** P1
- **Status:** pending
- The flagship resource: `create` (non-streaming) + `create_stream` (SSE). Full request params and response/chunk types.

## Key Insights
- Request params (`completions.py:250-286`): `messages`, `model`, `frequency_penalty`, `logprobs`, `max_completion_tokens`, `max_tokens`, `n`, `presence_penalty`, `response_format`, `seed`, `stop` (str | array), `stream`, `stream_options`, `temperature`, `tool_choice`, `tools`, `top_logprobs`, `top_p`, `user`. All optional except messages+model.
- Messages: roles `system`/`user`/`assistant`/`tool` (+`developer`). Assistant may carry `tool_calls`; tool messages carry `tool_call_id`. Content: string (keep v0.1 simple — string content; add content-parts array only if trivial). **Decision:** support `content: String` + assistant `tool_calls` + tool `tool_call_id`; multimodal content-parts OUT for v0.1 (document).
- `response_format`: `{ "type": "text" | "json_object" | "json_schema", "json_schema": {...} }` — model as enum with a `JsonSchema` variant carrying `serde_json::Value` schema (expose only; no runtime parsing per scope).
- `tools`: `{ "type": "function", "function": { name, description?, parameters: Value } }`. `tool_choice`: `"none"|"auto"|"required"` or `{ "type":"function","function":{"name":...}}`.
- Response `ChatCompletion` (`chat_completion.py`): `id, choices[], created, model, object, service_tier?, system_fingerprint?, usage?`. `Choice { index, message, finish_reason: "stop"|"length"|"tool_calls"|"content_filter"|"function_call", logprobs? }`. `usage: { prompt_tokens, completion_tokens, total_tokens, ... }`.
- Chunk `ChatCompletionChunk`: `choices[].delta { role?, content?, tool_calls? }`, `finish_reason?`, plus optional `usage` when `stream_options.include_usage`.
- `stream_options`: `{ include_usage: bool }`.

## Requirements
**Functional**
- `client.chat().completions().create(request) -> Result<ChatCompletion, OpenAIError>`.
- `client.chat().completions().create_stream(request) -> Result<EventStream<ChatCompletionChunk>, OpenAIError>` (sets `stream=true`, `stream_options.include_usage` passthrough).
- Ergonomic request builder for messages (`Message::system/user/assistant/tool`).

**Non-functional**
- Types split across files, each < 800 lines. Optional fields skip-serialize when None.

## Architecture
```
src/types/
├── mod.rs
├── common.rs                 # Usage, Role, FinishReason, ResponseFormat, Tool, ToolChoice
└── chat.rs                   # ChatCompletionRequest, Message, ChatCompletion, Choice, Chunk types
src/resources/
├── mod.rs
└── chat.rs                   # Chat / Completions accessors + create / create_stream
```

## Related Code Files
**Create:** `src/types/mod.rs`, `src/types/common.rs`, `src/types/chat.rs`, `src/resources/mod.rs`, `src/resources/chat.rs`
**Modify:** `src/client.rs` (`chat()` accessor), `src/lib.rs` (re-exports)

## Implementation Steps
1. `common.rs`:
   - `enum Role { System, User, Assistant, Tool, Developer }` (`#[serde(rename_all="lowercase")]`).
   - `enum FinishReason { Stop, Length, ToolCalls, ContentFilter, FunctionCall }` (`snake_case`, `#[serde(other)] Unknown`).
   - `struct Usage { prompt_tokens, completion_tokens, total_tokens, .. }` (`#[serde(default)]`).
   - `enum ResponseFormat { Text, JsonObject, JsonSchema { json_schema: Value } }` tagged by `type`.
   - `struct Tool { r#type:"function", function: FunctionDef { name, description?, parameters: Value } }`.
   - `enum ToolChoice { Auto, None, Required, Function { function: {name} } }` (untagged/custom serialize).
2. `chat.rs` (types):
   - `struct Message { role, content: Option<String>, name?, tool_calls?: Vec<ToolCall>, tool_call_id? }` + constructors.
   - `struct ChatCompletionRequest { model, messages, + all optional params, stream?, stream_options? }` with `#[serde(skip_serializing_if="Option::is_none")]`. Provide `ChatCompletionRequest::builder(model, messages)`.
   - Response: `ChatCompletion`, `Choice`, `ChatCompletionMessage { role, content?, tool_calls?, refusal? }`, `ToolCall { id, type, function: { name, arguments: String } }`.
   - Chunk: `ChatCompletionChunk`, chunk `Choice { index, delta, finish_reason? }`, `ChoiceDelta { role?, content?, tool_calls?: Vec<ChoiceDeltaToolCall> }`, `ChoiceDeltaToolCall { index, id?, function?: { name?, arguments? } }`.
3. `resources/chat.rs`:
   - `Chat<'a>` → `.completions() -> Completions<'a>`.
   - `create`: `POST /chat/completions` via `client.execute::<ChatCompletion>` with `stream` forced None/false.
   - `create_stream`: set `stream=Some(true)`, `execute_raw` → `EventStream::<ChatCompletionChunk>::new(response)`.
4. `cargo build` + a doctest example on `create`.

## Todo List
- [ ] common.rs: Role, FinishReason, Usage, ResponseFormat, Tool, ToolChoice
- [ ] chat.rs types: Message (+constructors) + ChatCompletionRequest (+builder)
- [ ] chat.rs types: ChatCompletion/Choice/Message/ToolCall responses
- [ ] chat.rs types: Chunk/Delta/DeltaToolCall
- [ ] resources/chat.rs: create + create_stream
- [ ] client.chat() accessor + re-exports
- [ ] cargo build clean

## Success Criteria
- Round-trips a real OpenAI chat response JSON into `ChatCompletion` (fixture test).
- `create_stream` yields `ChatCompletionChunk`s and terminates on `[DONE]`.
- Request with `tools` + `response_format` serializes to expected JSON (skip-none verified).

## Risk Assessment
- `stop` polymorphism (string vs array): model as `enum Stop { One(String), Many(Vec<String>) }` `#[serde(untagged)]`.
- `tool_choice` serialization: custom `Serialize` to emit bare strings or object.

## Security Considerations
- None beyond auth already handled in request layer.

## Next Steps
- Phase 05 reuses `common.rs` types (Usage) and the request pipeline.
