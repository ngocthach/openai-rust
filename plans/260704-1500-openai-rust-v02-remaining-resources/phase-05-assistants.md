# Phase 05 — Assistants (Beta v2)

**Owner:** subagent. **Parallel group:** B.
**File ownership (create ONLY these):**
- `src/types/assistants.rs`
- `src/resources/assistants.rs`
- `tests/assistants-tests.rs`

Do NOT edit shared files. See compile note in `plan.md`.

## Context Links
- Python: `resources/beta/assistants.py`, `resources/beta/threads/threads.py`,
  `.../messages.py`, `.../runs/runs.py`, `.../runs/steps.py`.
- Types under `types/beta/`.
- Follow `files.rs`/`chat.rs` patterns.

## CRITICAL — Beta header
Every request in this phase MUST send `OpenAI-Beta: assistants=v2`
(`assistants.py:165` etc.). Add it via `RequestOptions.extra_headers`:
```rust
const BETA: (&str,&str) = ("OpenAI-Beta", "assistants=v2");
// options.extra_headers.push((BETA.0.into(), BETA.1.into()));
```
Caller-supplied header overrides (push caller header last), matching Python merge.

## Overview
- **Priority:** P2. **Status:** pending.
- Assistants CRUD+list; threads create/retrieve/update/delete; messages CRUD+list;
  runs create/retrieve/update/list/cancel/submit_tool_outputs; run steps list/retrieve.
- OUT of scope: streaming runs, `create_and_run` streaming, vector-store polling.

## Endpoints (all send beta header)
### Assistants
| create | POST `/assistants` | retrieve GET `/assistants/{id}` | update POST `/assistants/{id}` | list GET `/assistants` | delete DELETE `/assistants/{id}` |
### Threads
| create POST `/threads` | retrieve GET `/threads/{id}` | update POST `/threads/{id}` | delete DELETE `/threads/{id}` |
### Messages
| create POST `/threads/{tid}/messages` | retrieve GET `/threads/{tid}/messages/{mid}` | update POST `/threads/{tid}/messages/{mid}` | list GET `/threads/{tid}/messages` | delete DELETE `/threads/{tid}/messages/{mid}` |
### Runs
| create POST `/threads/{tid}/runs` | retrieve GET `/threads/{tid}/runs/{rid}` | update POST `/threads/{tid}/runs/{rid}` | list GET `/threads/{tid}/runs` | cancel POST `/threads/{tid}/runs/{rid}/cancel` | submit_tool_outputs POST `/threads/{tid}/runs/{rid}/submit_tool_outputs` |
### Steps
| retrieve GET `/threads/{tid}/runs/{rid}/steps/{sid}` | list GET `/threads/{tid}/runs/{rid}/steps` |

List query: `after?,before?,limit?(1-100 def20),order?(asc|desc)`. Messages list also
`run_id?`. Steps list/retrieve also `include?` (=`step_details.tool_calls[*].file_search.results[*].content`).

## Bodies (scalars typed; polymorphic → serde_json::Value; skip-none)
- assistant create: `model` (req), `name?`,`description?`,`instructions?`,`metadata?`,
  `temperature?:f64`,`top_p?:f64`,`reasoning_effort?`,`response_format?`(Value),
  `tools?`(Vec<Value>),`tool_resources?`(Value). update: same, `model?`.
- thread create: `messages?`(Vec<Value>),`metadata?`,`tool_resources?`(Value).
- message create: `role` (`user|assistant`, req), `content` (`String|Vec<Value>`, req),
  `attachments?`(Vec<Value>),`metadata?`. update: `metadata?`.
- run create: `assistant_id` (req), `model?`,`instructions?`,`additional_instructions?`,
  `additional_messages?`(Vec<Value>),`metadata?`,`temperature?`,`top_p?`,
  `max_completion_tokens?:i64`,`max_prompt_tokens?:i64`,`parallel_tool_calls?:bool`,
  `reasoning_effort?`,`response_format?`(Value),`tool_choice?`(Value),`tools?`(Vec<Value>),
  `truncation_strategy?`(Value); query `include?`. update: `metadata?`.
- submit_tool_outputs: `tool_outputs` (req `Vec<{output:String,tool_call_id:String}>`).

## Types (typed top-level; Value for deep polymorphic — see below)
- `Assistant` (`HasId`): `id`,`created_at:i64`,`model`,`object`,`name?`,`description?`,
  `instructions?`,`metadata?`,`temperature?`,`top_p?`,`tools:Vec<Value>`,
  `response_format?`(Value),`tool_resources?`(Value).
- `Thread` (`HasId`): `id`,`created_at:i64`,`object`,`metadata?`,`tool_resources?`(Value).
- `Message` (`HasId`): `id`,`thread_id`,`created_at:i64`,`object`,
  `role`(`user|assistant`),`status`(`in_progress|incomplete|completed`),
  `content:Vec<MessageContentBlock>`,`assistant_id?`,`run_id?`,`attachments?`(Vec<Value>),
  `completed_at?:i64`,`incomplete_at?:i64`,`incomplete_details?`(Value),`metadata?`.
  `MessageContentBlock` = `#[serde(tag="type")]`: `Text{text:{value:String,annotations:Vec<Value>}}`
  + `#[serde(other)] Other` fallback (image_file/image_url/refusal → keep minimal).
- `Run` (`HasId`): `id`,`thread_id`,`assistant_id`,`created_at:i64`,`object`,
  `status:RunStatus`,`model`,`instructions`,`parallel_tool_calls:bool`,`tools:Vec<Value>`,
  optional `started_at`,`expires_at`,`cancelled_at`,`failed_at`,`completed_at` (i64),
  `last_error?`(Value),`incomplete_details?`(Value),`required_action?`(Value),
  `response_format?`(Value),`tool_choice?`(Value),`truncation_strategy?`(Value),
  `usage?{completion_tokens,prompt_tokens,total_tokens}`,`temperature?`,`top_p?`,
  `max_completion_tokens?`,`max_prompt_tokens?`,`metadata?`.
  `RunStatus` (verbatim): `queued|in_progress|requires_action|cancelling|cancelled|failed|completed|incomplete|expired`.
- `RunStep` (`HasId`): `id`,`run_id`,`thread_id`,`assistant_id`,`created_at:i64`,`object`,
  `type`(`message_creation|tool_calls`),`status`(`in_progress|cancelled|failed|completed|expired`),
  `step_details:Value`,`last_error?`(Value),`usage?`,`metadata?`, optional `*_at` fields.
- `AssistantDeleted`/`ThreadDeleted`/`MessageDeleted`: `{id,deleted:bool,object}`.

## Accessor signatures (phase-08 will add)
```rust
impl Client { pub fn beta(&self) -> Beta; }
// Beta::assistants() -> Assistants ; Beta::threads() -> Threads
// Assistants: create/retrieve/update/list/list_all/delete
// Threads: create/retrieve/update/delete; threads.messages()->Messages; threads.runs()->Runs
// Messages: create(thread_id,..)/retrieve/update/list/list_all/delete
// Runs: create/retrieve/update/list/cancel/submit_tool_outputs; runs.steps()->Steps
// Steps: list(thread_id,run_id,..)/retrieve(thread_id,run_id,step_id)
```
Nested resources hold `Client` + optional parent ids (or take ids as args — KISS: take
ids as method args, no stored parent id).

## Implementation Steps
1. `types/assistants.rs`: response types (`Assistant`,`Thread`,`Message`,`Run`,`RunStep`,
   deleted markers), `RunStatus`, `MessageContentBlock`, params structs, `HasId` impls.
2. `resources/assistants.rs`: `Beta`→`Assistants`/`Threads`→`Messages`/`Runs`→`Steps`;
   every method injects the beta header. Keep file < 800 lines (split if needed).
3. `tests/assistants-tests.rs`: wiremock asserting `OpenAI-Beta: assistants=v2` on each,
   run status deserialization, submit_tool_outputs body, list pagination.

## Todo List
- [ ] Response types + `RunStatus` + `MessageContentBlock` + `HasId`
- [ ] Params structs (scalars typed, polymorphic Value)
- [ ] `Beta`/`Assistants`/`Threads`/`Messages`/`Runs`/`Steps` resources + beta header
- [ ] wiremock tests (assert beta header present)
- [ ] local `cargo test`; files < 800 lines

## Success Criteria
- Every request carries `OpenAI-Beta: assistants=v2`; all paths/verbs correct;
  `RunStatus` deserializes; nested resource ergonomics compile.

## Risk Assessment
- Deep polymorphic content: Value fallback keeps scope bounded (YAGNI). Type only
  `text` block + status/role enums.
- File size — split into submodules under your own new files if > 800 lines.

## Security Considerations
- None new.

## Next Steps
- Phase-08 wires `client.beta()`. Independent of other group-B phases.
