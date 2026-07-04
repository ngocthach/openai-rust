//! Integration tests for the fine-tuning jobs resource against a wiremock
//! server: request shape, response parsing, and cursor pagination.

use openai_compat::types::fine_tuning::{
    AutoOr, DpoHyperparameters, DpoMethod, FineTuningJobRequest, FineTuningJobStatus,
    FineTuningMethod, Hyperparameters, SupervisedMethod,
};
use openai_compat::Client;
use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-test-key")
        .base_url(format!("{}/v1", server.uri()))
        .build()
        .unwrap()
}

fn job_body(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "object": "fine_tuning.job",
        "created_at": 1_700_000_000_i64,
        "model": "gpt-4o-mini-2024-07-18",
        "organization_id": "org-test",
        "result_files": [],
        "seed": 42,
        "status": "queued",
        "training_file": "file-train",
        "hyperparameters": {"n_epochs": "auto"},
        "trained_tokens": null,
        "validation_file": null
    })
}

#[tokio::test]
async fn create_job_sends_body_and_parses_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/fine_tuning/jobs"))
        .and(header("authorization", "Bearer sk-test-key"))
        .and(body_partial_json(json!({
            "model": "gpt-4o-mini-2024-07-18",
            "training_file": "file-train",
            "suffix": "my-suffix",
            "seed": 7,
            "method": {
                "type": "supervised",
                "supervised": {
                    "hyperparameters": {"n_epochs": 3, "learning_rate_multiplier": "auto"}
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(job_body("ftjob-abc")))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let request = FineTuningJobRequest::new("gpt-4o-mini-2024-07-18", "file-train")
        .suffix("my-suffix")
        .seed(7)
        .method(FineTuningMethod::supervised(SupervisedMethod {
            hyperparameters: Some(Hyperparameters {
                batch_size: None,
                learning_rate_multiplier: Some(AutoOr::Auto),
                n_epochs: Some(AutoOr::Value(3)),
            }),
        }));

    let job = client.fine_tuning().jobs().create(request).await.unwrap();
    assert_eq!(job.id, "ftjob-abc");
    assert_eq!(job.status, Some(FineTuningJobStatus::Queued));
    assert_eq!(job.model, "gpt-4o-mini-2024-07-18");
}

#[tokio::test]
async fn create_job_serializes_dpo_method() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/fine_tuning/jobs"))
        .and(body_partial_json(json!({
            "model": "gpt-4o",
            "training_file": "file-train",
            "method": {
                "type": "dpo",
                "dpo": {"hyperparameters": {"beta": 0.1, "n_epochs": "auto"}}
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(job_body("ftjob-dpo")))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let request = FineTuningJobRequest::new("gpt-4o", "file-train").method(
        FineTuningMethod::dpo(DpoMethod {
            hyperparameters: Some(DpoHyperparameters {
                batch_size: None,
                beta: Some(AutoOr::Value(0.1)),
                learning_rate_multiplier: None,
                n_epochs: Some(AutoOr::Auto),
            }),
        }),
    );

    let job = client.fine_tuning().jobs().create(request).await.unwrap();
    assert_eq!(job.id, "ftjob-dpo");
}

#[tokio::test]
async fn retrieve_job_parses_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/fine_tuning/jobs/ftjob-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(job_body("ftjob-abc")))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let job = client
        .fine_tuning()
        .jobs()
        .retrieve("ftjob-abc")
        .await
        .unwrap();
    assert_eq!(job.id, "ftjob-abc");
    assert_eq!(job.seed, Some(42));
}

#[tokio::test]
async fn cancel_job_posts_and_parses() {
    let server = MockServer::start().await;
    let mut cancelled = job_body("ftjob-abc");
    cancelled["status"] = json!("cancelled");
    Mock::given(method("POST"))
        .and(path("/v1/fine_tuning/jobs/ftjob-abc/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(cancelled))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let job = client
        .fine_tuning()
        .jobs()
        .cancel("ftjob-abc")
        .await
        .unwrap();
    assert_eq!(job.status, Some(FineTuningJobStatus::Cancelled));
}

#[tokio::test]
async fn list_events_parses_and_sends_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/fine_tuning/jobs/ftjob-abc/events"))
        .and(query_param("limit", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "has_more": false,
            "data": [
                {
                    "id": "ftevent-1",
                    "object": "fine_tuning.job.event",
                    "created_at": 1_700_000_100_i64,
                    "level": "info",
                    "message": "Job started",
                    "type": "message"
                },
                {
                    "id": "ftevent-2",
                    "object": "fine_tuning.job.event",
                    "created_at": 1_700_000_200_i64,
                    "level": "info",
                    "message": "Step 1",
                    "type": "metrics",
                    "data": {"step": 1, "train_loss": 0.5}
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = openai_compat::types::fine_tuning::FineTuningPageParams {
        after: None,
        limit: Some(2),
    };
    let events = client
        .fine_tuning()
        .jobs()
        .list_events("ftjob-abc", Some(params))
        .await
        .unwrap();
    assert_eq!(events.data.len(), 2);
    assert_eq!(events.data[0].id, "ftevent-1");
    assert_eq!(events.data[1].event_type.as_deref(), Some("metrics"));
}

#[tokio::test]
async fn list_checkpoints_parses_metrics() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/fine_tuning/jobs/ftjob-abc/checkpoints"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "has_more": false,
            "data": [{
                "id": "ftckpt-1",
                "object": "fine_tuning.job.checkpoint",
                "created_at": 1_700_000_300_i64,
                "fine_tuned_model_checkpoint": "ft:gpt-4o:ckpt-1",
                "fine_tuning_job_id": "ftjob-abc",
                "step_number": 100,
                "metrics": {
                    "step": 100.0,
                    "train_loss": 0.42,
                    "valid_loss": 0.5
                }
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let checkpoints = client
        .fine_tuning()
        .jobs()
        .list_checkpoints("ftjob-abc", None)
        .await
        .unwrap();
    assert_eq!(checkpoints.data.len(), 1);
    let ckpt = &checkpoints.data[0];
    assert_eq!(ckpt.id, "ftckpt-1");
    assert_eq!(ckpt.step_number, 100);
    assert_eq!(ckpt.metrics.as_ref().unwrap().train_loss, Some(0.42));
}

#[tokio::test]
async fn list_jobs_sends_metadata_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/fine_tuning/jobs"))
        .and(query_param("metadata[env]", "prod"))
        .and(query_param("limit", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "has_more": false,
            "data": [job_body("ftjob-abc")]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("env".to_string(), "prod".to_string());
    let params = openai_compat::types::fine_tuning::FineTuningJobListParams {
        after: None,
        limit: Some(5),
        metadata: Some(metadata),
    };
    let jobs = client
        .fine_tuning()
        .jobs()
        .list(Some(params))
        .await
        .unwrap();
    assert_eq!(jobs.data.len(), 1);
}
