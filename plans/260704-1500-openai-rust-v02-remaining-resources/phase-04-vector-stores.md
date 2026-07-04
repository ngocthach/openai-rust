# Phase 04 — Vector Stores

**Owner:** subagent. **Parallel group:** B.
**File ownership (create ONLY these):**
- `src/types/vector_stores.rs`
- `src/resources/vector_stores.rs`
- `tests/vector-stores-tests.rs`

Do NOT edit shared files. See compile note in `plan.md`.

## Context Links
- Python: `resources/vector_stores/vector_stores.py`, `.../files.py`, `.../file_batches.py`.
- Types: `types/vector_store.py`, `vector_store_create_params.py`,
  `vector_store_search_params.py`, `vector_store_search_response.py`,
  `types/vector_stores/vector_store_file.py`, `vector_store_file_batch.py`,
  `file_create_params.py`, `file_batch_create_params.py`,
  `file_chunking_strategy*.py`.
- Follow `files.rs`/`models.rs` patterns. Bearer only, no beta header.

## Overview
- **Priority:** P2. **Status:** pending.
- Stores CRUD+list+search; files create/retrieve/list/delete(+content); file_batches
  create/retrieve/cancel/list_files. Poll helpers OUT of scope.

## Endpoints
### Stores (`vector_stores.py`)
| Method | HTTP | Path |
|--------|------|------|
| create | POST | `/vector_stores` |
| retrieve | GET | `/vector_stores/{id}` |
| update | POST | `/vector_stores/{id}` |
| list | GET | `/vector_stores` (query `after,before,limit,order`) |
| delete | DELETE | `/vector_stores/{id}` |
| search | POST | `/vector_stores/{id}/search` |

### Files (`files.py`)
| create | POST | `/vector_stores/{id}/files` |
| retrieve | GET | `/vector_stores/{id}/files/{file_id}` |
| update | POST | `/vector_stores/{id}/files/{file_id}` |
| list | GET | `/vector_stores/{id}/files` (query `after,before,filter,limit,order`) |
| delete | DELETE | `/vector_stores/{id}/files/{file_id}` |
| content | GET | `/vector_stores/{id}/files/{file_id}/content` |

### File batches (`file_batches.py`)
| create | POST | `/vector_stores/{id}/file_batches` |
| retrieve | GET | `/vector_stores/{id}/file_batches/{batch_id}` |
| cancel | POST | `/vector_stores/{id}/file_batches/{batch_id}/cancel` |
| list_files | GET | `/vector_stores/{id}/file_batches/{batch_id}/files` (query `after,before,filter,limit,order`) |

Note: `update` verbs are POST (not PATCH). `filter` enum = `in_progress|completed|failed|cancelled`. `order` = `asc|desc`.

## Bodies
- store create: `name?`, `description?`, `file_ids?:Vec<String>`, `metadata?`,
  `expires_after?{anchor:"last_active_at",days:i32}`, `chunking_strategy?`.
- store update: `name?`, `metadata?`, `expires_after?`.
- store search: `query` (`String|Vec<String>`, req), `filters?` (Value), `max_num_results?:u32`,
  `ranking_options?` (Value), `rewrite_query?:bool`.
- file create: `file_id` (req), `attributes?` (Value), `chunking_strategy?`.
- file update: `attributes` (Value, required-nullable).
- file_batch create: `file_ids?:Vec<String>` XOR `files?:Vec<{file_id,attributes?,chunking_strategy?}>`,
  `attributes?` (Value), `chunking_strategy?`.

`chunking_strategy` (`#[serde(tag="type")]`): request `{type:"auto"}` |
`{type:"static", static:{max_chunk_size_tokens:u32 (def 800), chunk_overlap_tokens:u32 (def 400)}}`.
response adds `{type:"other"}` variant.

## Types
- `VectorStore` (`HasId`): `id`, `created_at:i64`, `name`, `object`, `status`
  (`expired|in_progress|completed`), `usage_bytes:i64`,
  `file_counts{in_progress,completed,failed,cancelled,total}` (i32), `last_active_at?:i64`,
  `metadata?`, `expires_after?{anchor,days}`, `expires_at?:i64`.
- `VectorStoreFile` (`HasId`): `id`, `created_at:i64`, `vector_store_id`, `object`,
  `status` (`in_progress|completed|cancelled|failed`), `usage_bytes:i64`,
  `last_error?{code:server_error|unsupported_file|invalid_file,message}`,
  `attributes?` (Value), `chunking_strategy?` (FileChunkingStrategy).
- `VectorStoreFileBatch` (`HasId`): `id`, `created_at:i64`, `vector_store_id`,
  `object` (literal `vector_store.files_batch`), `status`
  (`in_progress|completed|cancelled|failed`), `file_counts{..}` (i32).
- `VectorStoreSearchResponse`: `file_id`, `filename`, `score:f64`, `attributes?` (Value),
  `content:Vec<{text:String,type:String}>`.
- `VectorStoreDeleted`/`VectorStoreFileDeleted`: `{id,deleted:bool,object}`.
- `FileContentResponse`: `text?`, `type?`.
- search + content return a simple `{data:[...]}` page (non-cursor) — deserialize as `List<T>`.

## Accessor signatures (phase-08 will add)
```rust
impl Client { pub fn vector_stores(&self) -> VectorStores; }
// VectorStores: create(params)->VectorStore, retrieve(&str)->VectorStore,
//   update(id,params)->VectorStore, list(Option<params>)->List<VectorStore>,
//   list_all()->Vec<..>, delete(&str)->VectorStoreDeleted,
//   search(id, params)->List<VectorStoreSearchResponse>,
//   files(&self)->VectorStoreFiles, file_batches(&self)->VectorStoreFileBatches
// VectorStoreFiles: create(store_id,params)->VectorStoreFile, retrieve(store_id,file_id)->..,
//   update(store_id,file_id,attributes)->.., list(store_id,Option<params>)->List<..>,
//   delete(store_id,file_id)->VectorStoreFileDeleted, content(store_id,file_id)->List<FileContentResponse>
// VectorStoreFileBatches: create(store_id,params)->VectorStoreFileBatch,
//   retrieve(store_id,batch_id)->.., cancel(store_id,batch_id)->.., list_files(store_id,batch_id,Option<params>)->List<VectorStoreFile>
```

## Implementation Steps
1. `types/vector_stores.rs`: all response types, `FileChunkingStrategy` (req+resp enums),
   params structs, `HasId` impls (VectorStore, VectorStoreFile, VectorStoreFileBatch).
2. `resources/vector_stores.rs`: `VectorStores` + nested `VectorStoreFiles`,
   `VectorStoreFileBatches`. Keep file < 800 lines (split types vs resource already).
3. `tests/vector-stores-tests.rs`: wiremock per endpoint incl. search POST body and
   chunking_strategy serialization.

## Todo List
- [ ] Response + params + chunking_strategy types + `HasId`
- [ ] `VectorStores` + `VectorStoreFiles` + `VectorStoreFileBatches`
- [ ] wiremock tests (incl. search body, static chunking)
- [ ] local `cargo test`; verify files < 800 lines

## Success Criteria
- All 16 endpoints correct verb/path; POST-update honored; search returns typed results;
  chunking_strategy round-trips auto/static.

## Risk Assessment
- File size: if `vector_stores.rs` nears 800 lines, split resource into
  `vector_stores.rs` re-exporting submodules (still your own new files only).

## Security Considerations
- None new.

## Next Steps
- Phase-08 wires `client.vector_stores()`. Independent of other group-B phases.
