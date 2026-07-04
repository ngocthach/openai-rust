# Phase 02 — Batches + Resumable Uploads

**Owner:** subagent. **Parallel group:** B.
**File ownership (create ONLY these):**
- `src/types/batches.rs`, `src/types/uploads.rs`
- `src/resources/batches.rs`, `src/resources/uploads.rs`
- `tests/batches-uploads-tests.rs`

Do NOT edit `Cargo.toml`, `lib.rs`, `src/types/mod.rs`, `src/resources/mod.rs`,
`client.rs`. Phase-08 wires these. See compile note in `plan.md`.

## Context Links
- Python: `resources/batches.py` (create L47-133, retrieve L135-170, list L172-223,
  cancel L225-263), `resources/uploads/uploads.py` (create L168-252, cancel L254-292,
  complete L294-357), `resources/uploads/parts.py` (create/add_part L45-101).
- Types: `types/batch.py`, `types/upload.py`, `types/uploads/upload_part.py`, and
  `*_params.py` in those dirs.
- Follow existing patterns: `src/resources/files.rs`, `src/types/files.rs` (multipart
  via `MultipartField`, `RequestOptions`, `Client::execute`, `paginate_all`, `HasId`).

## Overview
- **Priority:** P2. **Status:** pending.
- Batch job CRUD + list; resumable multi-part uploads (create → add_part → complete/cancel).

## Endpoints

### Batches (JSON)
| Method | HTTP | Path | Body / Query |
|--------|------|------|--------------|
| create | POST | `/batches` | body: `completion_window` (req, only `"24h"`), `endpoint` (req), `input_file_id` (req), `metadata?`, `output_expires_after?{anchor:"created_at",seconds}` |
| retrieve | GET | `/batches/{batch_id}` | — |
| list | GET | `/batches` | query: `after?`, `limit?` (1-100, def 20) |
| cancel | POST | `/batches/{batch_id}/cancel` | — |

`endpoint` allowed values (verbatim): `/v1/responses`, `/v1/chat/completions`,
`/v1/embeddings`, `/v1/completions`, `/v1/moderations`, `/v1/images/generations`,
`/v1/images/edits`, `/v1/videos`.

### Uploads
| Method | HTTP | Path | Body |
|--------|------|------|------|
| create | POST | `/uploads` (JSON) | `bytes` (req int), `filename` (req), `mime_type` (req), `purpose` (req), `expires_after?{anchor:"created_at",seconds}` |
| add_part | POST | `/uploads/{upload_id}/parts` (**MULTIPART**) | file field `data` = raw chunk bytes |
| complete | POST | `/uploads/{upload_id}/complete` (JSON) | `part_ids` (req `Vec<String>`), `md5?` |
| cancel | POST | `/uploads/{upload_id}/cancel` | — |

`purpose` enum: `assistants|batch|fine-tune|vision|user_data|evals`. Default chunk
size const `DEFAULT_PART_SIZE = 64 * 1024 * 1024`.

## Types
- `Batch` (`#[non_exhaustive]`, `HasId`): `id`, `completion_window`, `created_at:i64`,
  `endpoint`, `input_file_id`, `object`, `status` (`validating|failed|in_progress|
  finalizing|completed|expired|cancelling|cancelled`), optional `cancelled_at`,
  `cancelling_at`, `completed_at`, `expired_at`, `expires_at`, `failed_at`,
  `finalizing_at`, `in_progress_at` (all `Option<i64>`), `error_file_id?`,
  `output_file_id?`, `errors?{data:Vec<BatchError>,object?}`, `metadata?`, `model?`,
  `request_counts?{completed,failed,total}`, `usage?` (Value or typed BatchUsage).
- `BatchError`: `code?`,`line?:i64`,`message?`,`param?` (all optional).
- `Upload` (`HasId`): `id`, `bytes:i64`, `created_at`, `expires_at`, `filename`,
  `object`, `purpose`, `status` (`pending|completed|cancelled|expired`), `file?:FileObject`.
  Reuse existing `crate::types::files::FileObject`.
- `UploadPart` (`HasId`): `id`, `created_at:i64`, `object`, `upload_id`.
- Optional body fields: `#[serde(skip_serializing_if = "Option::is_none")]`.

## Accessor signatures (phase-08 will add)
```rust
impl Client { pub fn batches(&self) -> Batches; pub fn uploads(&self) -> Uploads; }
// Batches: create(BatchCreateParams)->Batch, retrieve(&str)->Batch,
//   list(Option<BatchListParams>)->List<Batch>, list_all()->Vec<Batch>, cancel(&str)->Batch
// Uploads: create(UploadCreateParams)->Upload, add_part(upload_id,Vec<u8>)->UploadPart,
//   complete(upload_id, part_ids, md5:Option<&str>)->Upload, cancel(upload_id)->Upload
```
Design params as small structs or explicit args (KISS). Multipart `data` uses
`MultipartField::File { name:"data", filename:"part", bytes, content_type:None }`.

## Implementation Steps
1. `types/batches.rs`: `Batch`, `BatchError`, params structs, `HasId`.
2. `types/uploads.rs`: `Upload`, `UploadPart`, params structs, `HasId`.
3. `resources/batches.rs`: `Batches` struct + methods (mirror `files.rs`).
4. `resources/uploads.rs`: `Uploads` struct; add_part builds multipart form.
5. `tests/batches-uploads-tests.rs`: wiremock each endpoint (assert path/verb/body,
   multipart for add_part), plus a list-pagination test.

## Todo List
- [ ] `Batch`/`BatchError` types + params
- [ ] `Upload`/`UploadPart` types + params
- [ ] `Batches` resource (4 methods + list_all)
- [ ] `Uploads` resource (4 methods, multipart add_part)
- [ ] wiremock tests all endpoints
- [ ] locally `cargo test` (temp-wire, then revert shared edits)

## Success Criteria
- All 8 endpoints hit correct verb/path; add_part sends multipart; list follows cursor.

## Risk Assessment
- Multipart retry: rebuilt per attempt via `MultipartField` (already supported). OK.
- `usage`/`errors` nested shapes: use typed structs; fall back to `Value` if brittle.

## Security Considerations
- No secrets logged; bytes handled in memory (large uploads chunked by caller).

## Next Steps
- Phase-08 registers modules + accessors. Independent of other group-B phases.
