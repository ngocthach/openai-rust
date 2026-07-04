# Phase 03 — Fine-tuning Jobs

**Owner:** subagent. **Parallel group:** B.
**File ownership (create ONLY these):**
- `src/types/fine_tuning.rs`
- `src/resources/fine_tuning.rs`
- `tests/fine-tuning-tests.rs`

Do NOT edit shared files. See compile note in `plan.md`.

## Context Links
- Python: `resources/fine_tuning/jobs/jobs.py` (create L64/161-162, retrieve L187/214-215,
  list L226/258-259, cancel L279/304-305, list_events L316/347-348),
  `resources/fine_tuning/jobs/checkpoints.py` (list L46/77-78).
- Types: `types/fine_tuning/fine_tuning_job.py`, `fine_tuning_job_event.py`,
  `job_create_params.py`, `job_list_params.py`, `job_list_events_params.py`,
  `checkpoints/*`.
- Follow `src/resources/models.rs` / `files.rs` patterns.

## Overview
- **Priority:** P2. **Status:** pending.
- Create/retrieve/list/cancel fine-tuning jobs, list events, list checkpoints.
- OUT of scope: `pause`, `resume`, `alpha/*` graders, streaming.

## Endpoints (all JSON / bearer; no beta header)
| Method | HTTP | Path | Body / Query |
|--------|------|------|--------------|
| create | POST | `/fine_tuning/jobs` | body below |
| retrieve | GET | `/fine_tuning/jobs/{id}` | — |
| list | GET | `/fine_tuning/jobs` | query: `after?`, `limit?`, `metadata?` |
| cancel | POST | `/fine_tuning/jobs/{id}/cancel` | — |
| list_events | GET | `/fine_tuning/jobs/{id}/events` | query: `after?`, `limit?` |
| checkpoints.list | GET | `/fine_tuning/jobs/{id}/checkpoints` | query: `after?`, `limit?` |

**create body** (`job_create_params.py`): `model` (req, `str`), `training_file` (req, str),
`validation_file?`, `suffix?`, `seed?:i64`, `metadata?`, `integrations?` (Value/typed
`[{type:"wandb", wandb:{project(req),name?,entity?,tags?}}]`), `hyperparameters?`
(deprecated: `{batch_size, learning_rate_multiplier, n_epochs}` each `"auto"|number`),
`method?` (`{type:"supervised"|"dpo"|"reinforcement", supervised?/dpo?/reinforcement?:{hyperparameters:{...}}}`).
Recommend `serde_json::Value` for `hyperparameters`, `method`, `integrations` (deeply
polymorphic; YAGNI to fully type) with skip-none; type the scalar fields.

## Types
- `FineTuningJob` (`#[non_exhaustive]`, `HasId`): `id`, `created_at:i64`, `error?{code,message,param?}`,
  `fine_tuned_model?`, `finished_at?:i64`, `hyperparameters` (Value), `model`,
  `object`, `organization_id`, `result_files:Vec<String>`, `seed:i64`,
  `status` (`validating_files|queued|running|succeeded|failed|cancelled`),
  `trained_tokens?:i64`, `training_file`, `validation_file?`, `estimated_finish?:i64`,
  `integrations?` (Value), `metadata?`, `method?` (Value).
- `FineTuningJobEvent` (`HasId`): `id`, `created_at:i64`, `level`, `message`, `object`,
  `type?`, `data?` (Value).
- `FineTuningJobCheckpoint` (`HasId`): `id`, `created_at:i64`, `fine_tuned_model_checkpoint`,
  `fine_tuning_job_id`, `metrics` (Value or typed `{step,train_loss,...}`), `object`,
  `step_number:i64`.

## Accessor signatures (phase-08 will add)
```rust
impl Client { pub fn fine_tuning(&self) -> FineTuning; }
// FineTuning::jobs(&self) -> Jobs
// Jobs: create(JobCreateParams)->FineTuningJob, retrieve(&str)->FineTuningJob,
//   list(Option<params>)->List<FineTuningJob>, list_all()->Vec<..>, cancel(&str)->FineTuningJob,
//   list_events(id, Option<params>)->List<FineTuningJobEvent>,
//   checkpoints(&self)->Checkpoints  // Checkpoints::list(job_id,..)->List<FineTuningJobCheckpoint>
```

## Implementation Steps
1. `types/fine_tuning.rs`: response types + `JobCreateParams` + `HasId` impls.
2. `resources/fine_tuning.rs`: `FineTuning` → `Jobs` → `Checkpoints` nested structs.
3. `tests/fine-tuning-tests.rs`: wiremock for each endpoint incl. create-with-method body
   and events/checkpoints pagination.

## Todo List
- [ ] Response types (`FineTuningJob`, `Event`, `Checkpoint`) + `HasId`
- [ ] `JobCreateParams` (scalars typed, method/hyperparams/integrations as Value)
- [ ] `Jobs` + `Checkpoints` resources
- [ ] wiremock tests
- [ ] local `cargo test`

## Success Criteria
- Create serializes `model`/`training_file` + optional `method` Value; list/events/
  checkpoints follow cursor; status enum deserializes.

## Risk Assessment
- `method`/`hyperparameters` schema churn — Value fallback isolates it.

## Security Considerations
- None new.

## Next Steps
- Phase-08 wires `client.fine_tuning()`. Independent of other group-B phases.
