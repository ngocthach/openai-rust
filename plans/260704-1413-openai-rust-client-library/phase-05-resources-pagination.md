# Phase 05 — Remaining Resources + Pagination

## Context Links
- Reference: `openai-python/src/openai/pagination.py:71-101` (`SyncCursorPage`: data, has_more, `after`=last item id)
- Reference: `resources/embeddings.py:51-110` (create params)
- Reference: `resources/models.py` (list `/models`, retrieve/delete `/models/{model}`)
- Reference: `resources/moderations.py:48-90` (`POST /moderations`)
- Reference: `resources/completions.py:52-210` (legacy `POST /completions`, stream)
- Reference: `resources/images.py:130-210` (`POST /images/generations`)
- Reference: `resources/files.py:60-380` (list/retrieve/delete/content, multipart upload, `extract_files`)
- Reference: `resources/audio/speech.py:50-90` (`POST /audio/speech`, binary), `resources/audio/transcriptions.py:68-120` (multipart)

## Overview
- **Priority:** P2
- **Status:** pending
- All non-chat resources + generic cursor pagination helper. Each is thin over the phase-02 request pipeline.

## Key Insights
- **Pagination** (`pagination.py`): response `{ data: [T], has_more?: bool }`; next page = `after = data[-1].id`. Stop when `has_more == false` or data empty. Provide `List<T>` struct + `auto_paginate` that requires items expose `id`.
- **embeddings** `POST /embeddings`: `input` (string | array | token arrays), `model`, `dimensions?`, `encoding_format?` (`float`|`base64`), `user?`. Response `CreateEmbeddingResponse { data: [{ embedding: Vec<f32>|String, index, object }], model, usage, object }`.
- **models**: `list -> /models` (returns `{ object, data: [Model] }`, NOT cursor-paginated → SyncPage), `retrieve /models/{id}`, `delete /models/{id}` → `{ id, deleted, object }`.
- **moderations** `POST /moderations`: `input` (string|array), `model?`. Response `{ id, model, results: [{ flagged, categories, category_scores, .. }] }`.
- **completions (legacy)** `POST /completions`: `model`, `prompt` (string|array|tokens), `max_tokens?`, `temperature?`, `top_p?`, `n?`, `stop?`, `stream?`, `logprobs?`, `echo?`, `presence_penalty?`, `frequency_penalty?`, `seed?`, `stream_options?`, `user?`, `suffix?`, `best_of?`. Response `Completion { id, choices:[{text, index, logprobs?, finish_reason}], created, model, object, usage? }`. `create` + `create_stream` (reuse `EventStream<Completion>`).
- **images** `POST /images/generations`: `prompt`, `model?`, `n?`, `size?`, `quality?`, `response_format?` (`url`|`b64_json`), `user?`. Response `ImagesResponse { created, data: [{ url?, b64_json?, revised_prompt? }] }`. (Only `generate`; edits/variations OUT.)
- **files**:
  - `list /files` (SyncPage `{ data, object }`, optional `purpose`/`after`/`limit` query).
  - `retrieve /files/{id}` → `FileObject { id, bytes, created_at, filename, object, purpose, status?, ... }`.
  - `delete /files/{id}` → `{ id, deleted, object }`.
  - `content /files/{id}/content` → bytes (`execute_bytes`).
  - `create /files` multipart: fields `file` (bytes+filename) + `purpose`. Use `reqwest::multipart::Form`.
- **audio.speech** `POST /audio/speech`: `input`, `model`, `voice`, `response_format?` (mp3/opus/aac/flac/wav/pcm), `speed?`, `instructions?` → binary `Bytes` (`execute_bytes`).
- **audio.transcriptions** `POST /audio/transcriptions` multipart: `file`, `model`, `language?`, `prompt?`, `response_format?`, `temperature?` → `Transcription { text }` (or verbose json; v0.1 return `{ text }`).

## Requirements
**Functional**
- Resource accessors: `client.embeddings()`, `.models()`, `.moderations()`, `.completions()`, `.images()`, `.files()`, `.audio().speech()/.transcriptions()`.
- `List<T>` cursor page + `auto_paginate` helper returning `impl Stream<Item=Result<T>>` (or `Vec<T>` collect helper).

**Non-functional**
- Each resource file focused, < 800 lines. Reuse `common::Usage`.

## Architecture
```
src/pagination.rs               # List<T>, Page trait (id accessor), auto_paginate helper
src/types/
├── embeddings.rs  models.rs  moderations.rs  completions.rs  images.rs  files.rs  audio.rs
src/resources/
├── embeddings.rs  models.rs  moderations.rs  completions.rs  images.rs  files.rs  audio.rs
```

## Related Code Files
**Create:** `src/pagination.rs`, `src/types/{embeddings,models,moderations,completions,images,files,audio}.rs`, `src/resources/{embeddings,models,moderations,completions,images,files,audio}.rs`
**Modify:** `src/client.rs` (accessors), `src/resources/mod.rs`, `src/types/mod.rs`, `src/lib.rs`

## Implementation Steps
1. `pagination.rs`: `struct List<T> { data: Vec<T>, has_more: Option<bool>, object: Option<String>, first_id?, last_id? }`; `trait HasId { fn id(&self) -> Option<&str> }`; `Client::auto_paginate<T: HasId + DeserializeOwned>(path, query)` → `try_unfold` fetching pages via `after` cursor until `has_more==false`/empty.
2. embeddings: request/response types + `create`. Model `input` as `enum EmbeddingInput { Text(String), Texts(Vec<String>), Tokens(Vec<i64>), TokenArrays(Vec<Vec<i64>>) }` untagged; `embedding` field `enum { Floats(Vec<f32>), Base64(String) }` untagged.
3. models: `Model { id, created, owned_by, object }`; `list` (SyncPage), `retrieve`, `delete`.
4. moderations: types + `create`.
5. completions (legacy): types + `create` + `create_stream` (reuse EventStream).
6. images: types + `generate` (`POST /images/generations`).
7. files: types + list/retrieve/delete/content + `create` (multipart via `reqwest::multipart::Form::new().part("file", Part::bytes(..).file_name(..)).text("purpose", ..)`).
8. audio: `speech` (binary Bytes) + `transcriptions` (multipart, returns `{text}`).
9. Wire accessors on `client.rs`; `cargo build`.

## Todo List
- [ ] pagination.rs: List<T> + HasId + auto_paginate
- [ ] embeddings (types + create)
- [ ] models (list/retrieve/delete)
- [ ] moderations (create)
- [ ] completions legacy (create + create_stream)
- [ ] images (generate)
- [ ] files (list/retrieve/delete/content + multipart create)
- [ ] audio (speech binary + transcriptions multipart)
- [ ] client accessors + re-exports
- [ ] cargo build clean

## Success Criteria
- Each endpoint hits correct method+path (fixture/mock tests in phase 06).
- `auto_paginate` walks pages via `after` and stops on `has_more=false`.
- Multipart upload builds a valid `multipart/form-data` body; speech returns raw bytes.

## Risk Assessment
- Polymorphic `input`/`prompt`/`embedding`: untagged enums may mis-deserialize; order variants specific→general and add tests.
- Files `FileObject` field variance across providers: mark optional + `#[serde(default)]`.

## Security Considerations
- Multipart: stream file bytes; do not log contents.

## Next Steps
- Phase 06 adds mock tests + examples exercising these endpoints.
