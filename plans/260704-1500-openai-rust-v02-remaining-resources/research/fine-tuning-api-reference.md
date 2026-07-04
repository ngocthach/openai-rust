# Fine-Tuning Jobs + Checkpoints API — Rust Port Reference

All refs from `openai-python` repo. All endpoints require `bearer_auth`.

## Endpoints

| Method | HTTP | Path | jobs.py refs |
|---|---|---|---|
| create | POST | `/fine_tuning/jobs` | L161-185 |
| retrieve | GET | `/fine_tuning/jobs/{fine_tuning_job_id}` | L214-224 |
| list | GET | `/fine_tuning/jobs` | L258-277 |
| cancel | POST | `/fine_tuning/jobs/{fine_tuning_job_id}/cancel` | L304-314 |
| list_events | GET | `/fine_tuning/jobs/{fine_tuning_job_id}/events` | L347-365 |
| pause (bonus) | POST | `/fine_tuning/jobs/{fine_tuning_job_id}/pause` | L392-402 |
| resume (bonus) | POST | `/fine_tuning/jobs/{fine_tuning_job_id}/resume` | L429-439 |
| checkpoints.list | GET | `/fine_tuning/jobs/{fine_tuning_job_id}/checkpoints` | checkpoints.py L77-95 |

retrieve takes no body. cancel/pause/resume are POST with no body.

## create body params — `job_create_params.py` L17-99

- **model**: Required `String` (open-ended; accepts arbitrary ft model names).
- **training_file**: Required `String`.
- **hyperparameters** (DEPRECATED — use method): `{ batch_size: "auto"|int, learning_rate_multiplier: "auto"|float, n_epochs: "auto"|int }`
- **integrations**: `Option<Vec<Integration>>`. `Integration = { type: "wandb"; wandb: { project: String (req); entity?; name?; tags: Vec<String> } }`
- **metadata**: `Option<Map<String,String>>` (max 16 pairs, key ≤64, val ≤512)
- **method**: `{ type: "supervised"|"dpo"|"reinforcement"; dpo?; reinforcement?; supervised? }`
- **seed**: `Option<i64>`
- **suffix**: `Option<String>` (≤64 chars)
- **validation_file**: `Option<String>`

### method nested structures (param and response names identical)
- **SupervisedMethod** = `{ hyperparameters?: { batch_size, learning_rate_multiplier, n_epochs } }` each `"auto"|number|null`
- **DpoMethod** = `{ hyperparameters?: { batch_size, beta, learning_rate_multiplier, n_epochs } }` each `"auto"|number|null`
- **ReinforcementMethod** = `{ grader: Value (REQUIRED, 5-variant polymorphic — use serde_json::Value); hyperparameters?: { batch_size, compute_multiplier, eval_interval, eval_samples, learning_rate_multiplier, n_epochs, reasoning_effort?: "default"|"low"|"medium"|"high" } }`

## Query params
- **list**: `after?`, `limit?: i32`, `metadata?` (filter syntax `metadata[k]=v`)
- **list_events**: `after?`, `limit?: i32`
- **checkpoints.list**: `after?`, `limit?: i32`
- All list endpoints are cursor-paginated (`data`/`has_more`, cursor = last item id) → reuse `List<T>` + `HasId`.

## Response: FineTuningJob — `fine_tuning_job.py` L76-177
- id: String; created_at: i64; error: `Option<{code, message, param?}>`; fine_tuned_model: Option<String>;
  finished_at: Option<i64>; hyperparameters (required): `{batch_size, learning_rate_multiplier, n_epochs}` ("auto"|num|null);
  model: String; object: "fine_tuning.job"; organization_id: String; result_files: Vec<String>; seed: i64;
  status: enum [validating_files, queued, running, succeeded, failed, cancelled]; trained_tokens: Option<i64>;
  training_file: String; validation_file: Option<String>; estimated_finish: Option<i64>;
  integrations: Option<Vec<Value>>; metadata: Option<Map<String,String>>; method: Option<Method>

## Response: FineTuningJobEvent
- id, created_at: i64, level: enum [info, warn, error], message: String, object: "fine_tuning.job.event",
  data: Option<Value>, type: Option<enum [message, metrics]>

## Response: FineTuningJobCheckpoint
- id, created_at: i64, fine_tuned_model_checkpoint: String, fine_tuning_job_id: String,
  metrics: `{ full_valid_loss?, full_valid_mean_token_accuracy?, step?, train_loss?, train_mean_token_accuracy?, valid_loss?, valid_mean_token_accuracy? }` all Option<f64>,
  object: "fine_tuning.job.checkpoint", step_number: i64

## Rust design notes
- `"auto"|number|null` unions: shared untagged `enum AutoOr<T> { Auto, Value(T) }` (serialize Auto as "auto"), or Value.
- Value fallback justified for: ReinforcementMethod.grader, FineTuningJobEvent.data, integrations objects.
- model stays String everywhere.
