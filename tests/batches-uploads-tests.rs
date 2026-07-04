//! Integration tests for the batches and resumable uploads resources against a
//! local wiremock server: request shape (verb/path/body), multipart part
//! upload, and list pagination.

use openai_rust::types::batches::{BatchCreateParams, BatchListParams, BatchStatus};
use openai_rust::types::uploads::{UploadCreateParams, UploadStatus};
use openai_rust::Client;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-test-key")
        .base_url(format!("{}/v1", server.uri()))
        .max_retries(2)
        .build()
        .unwrap()
}

fn batch_body(id: &str, status: &str) -> serde_json::Value {
    json!({
        "id": id,
        "object": "batch",
        "endpoint": "/v1/chat/completions",
        "input_file_id": "file-in",
        "completion_window": "24h",
        "status": status,
        "created_at": 1,
        "in_progress_at": 2,
        "output_file_id": "file-out",
        "request_counts": {"total": 3, "completed": 2, "failed": 1},
        "errors": {"object": "list", "data": [
            {"code": "bad", "line": 4, "message": "oops", "param": "input"}
        ]},
        "metadata": {"customer": "acme"}
    })
}

#[tokio::test]
async fn batch_create_retrieve_and_cancel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/batches"))
        .and(body_partial_json(json!({
            "input_file_id": "file-in",
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(batch_body("batch-1", "validating")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/batches/batch-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(batch_body("batch-1", "completed")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/batches/batch-1/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(batch_body("batch-1", "cancelling")))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);

    let created = client
        .batches()
        .create(BatchCreateParams::new(
            "file-in",
            "/v1/chat/completions",
            "24h",
        ))
        .await
        .unwrap();
    assert_eq!(created.id, "batch-1");
    assert_eq!(created.status, BatchStatus::Validating);
    assert_eq!(created.request_counts.unwrap().total, 3);
    assert_eq!(created.errors.unwrap().data[0].code.as_deref(), Some("bad"));

    let retrieved = client.batches().retrieve("batch-1").await.unwrap();
    assert_eq!(retrieved.status, BatchStatus::Completed);
    assert_eq!(retrieved.output_file_id.as_deref(), Some("file-out"));
    assert_eq!(retrieved.in_progress_at, Some(2));

    let cancelled = client.batches().cancel("batch-1").await.unwrap();
    assert_eq!(cancelled.status, BatchStatus::Cancelling);
}

#[tokio::test]
async fn batch_unknown_status_deserializes_to_unknown() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/batches/batch-x"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(batch_body("batch-x", "some_future_status")),
        )
        .mount(&server)
        .await;

    let client = client_for(&server);
    let batch = client.batches().retrieve("batch-x").await.unwrap();
    assert_eq!(batch.status, BatchStatus::Unknown);
}

#[tokio::test]
async fn batch_list_single_page_with_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/batches"))
        .and(query_param("limit", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [batch_body("batch-1", "completed"), batch_body("batch-2", "in_progress")],
            "has_more": false
        })))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let page = client
        .batches()
        .list(Some(BatchListParams::default().limit(2)))
        .await
        .unwrap();
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[1].status, BatchStatus::InProgress);
}

#[tokio::test]
async fn batch_list_all_follows_after_cursor() {
    let server = MockServer::start().await;
    // Page 2 (has explicit `after`) — mount first so it wins the match.
    Mock::given(method("GET"))
        .and(path("/v1/batches"))
        .and(query_param("after", "batch-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [batch_body("batch-3", "completed")],
            "has_more": false
        })))
        .mount(&server)
        .await;
    // Page 1.
    Mock::given(method("GET"))
        .and(path("/v1/batches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [batch_body("batch-1", "completed"), batch_body("batch-2", "completed")],
            "has_more": true
        })))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let batches = client.batches().list_all().await.unwrap();
    assert_eq!(
        batches.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
        vec!["batch-1", "batch-2", "batch-3"]
    );
}

#[tokio::test]
async fn upload_create_add_part_and_complete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/uploads"))
        .and(body_partial_json(json!({
            "bytes": 12,
            "filename": "data.jsonl",
            "mime_type": "application/jsonl",
            "purpose": "batch"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "upload-1",
            "object": "upload",
            "bytes": 12,
            "created_at": 1,
            "expires_at": 2,
            "filename": "data.jsonl",
            "purpose": "batch",
            "status": "pending"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/uploads/upload-1/parts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "part-1",
            "object": "upload.part",
            "created_at": 3,
            "upload_id": "upload-1"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/uploads/upload-1/complete"))
        .and(body_partial_json(json!({
            "part_ids": ["part-1"],
            "md5": "abc123"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "upload-1",
            "object": "upload",
            "bytes": 12,
            "created_at": 1,
            "expires_at": 2,
            "filename": "data.jsonl",
            "purpose": "batch",
            "status": "completed",
            "file": {"id": "file-9", "object": "file", "filename": "data.jsonl"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);

    let upload = client
        .uploads()
        .create(UploadCreateParams::new(
            "data.jsonl",
            12,
            "application/jsonl",
            "batch",
        ))
        .await
        .unwrap();
    assert_eq!(upload.id, "upload-1");
    assert_eq!(upload.status, UploadStatus::Pending);

    let part = client
        .uploads()
        .add_part("upload-1", b"{\"a\":1}\n".to_vec())
        .await
        .unwrap();
    assert_eq!(part.id, "part-1");
    assert_eq!(part.upload_id, "upload-1");

    let completed = client
        .uploads()
        .complete("upload-1", vec!["part-1".to_string()], Some("abc123"))
        .await
        .unwrap();
    assert_eq!(completed.status, UploadStatus::Completed);
    assert_eq!(completed.file.unwrap().id, "file-9");

    // Verify the part upload used multipart/form-data with a `data` field.
    let requests = server.received_requests().await.unwrap();
    let part_req = requests
        .iter()
        .find(|r| r.url.path() == "/v1/uploads/upload-1/parts")
        .unwrap();
    let content_type = part_req
        .headers
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.starts_with("multipart/form-data"));
    let body = String::from_utf8_lossy(&part_req.body);
    assert!(body.contains("name=\"data\""));
    assert!(body.contains("{\"a\":1}"));
}

#[tokio::test]
async fn upload_cancel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/uploads/upload-1/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "upload-1",
            "object": "upload",
            "bytes": 12,
            "created_at": 1,
            "expires_at": 2,
            "filename": "data.jsonl",
            "purpose": "batch",
            "status": "cancelled"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let cancelled = client.uploads().cancel("upload-1").await.unwrap();
    assert_eq!(cancelled.status, UploadStatus::Cancelled);
}
