//! Integration tests for the vector stores resource against a wiremock server.
//! Verifies request shapes, the required `OpenAI-Beta: assistants=v2` header,
//! POST-update/search bodies, and sub-resource routing.

use openai_compat::types::vector_stores::{
    SearchQuery, VectorStoreCreateParams, VectorStoreFileBatchCreateParams,
    VectorStoreFileCreateParams, VectorStoreFileStatus, VectorStoreSearchParams, VectorStoreStatus,
};
use openai_compat::Client;
use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-test-key")
        .base_url(format!("{}/v1", server.uri()))
        .max_retries(1)
        .build()
        .unwrap()
}

fn store_body() -> serde_json::Value {
    json!({
        "id": "vs_123",
        "object": "vector_store",
        "created_at": 1700000000,
        "name": "Support FAQ",
        "status": "completed",
        "usage_bytes": 4096,
        "file_counts": {
            "in_progress": 0, "completed": 2, "failed": 0, "cancelled": 0, "total": 2
        },
        "metadata": {"team": "support"},
        "expires_after": {"anchor": "last_active_at", "days": 7}
    })
}

#[tokio::test]
async fn store_create_sends_beta_header_and_parses_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/vector_stores"))
        .and(header("authorization", "Bearer sk-test-key"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_partial_json(json!({
            "name": "Support FAQ",
            "file_ids": ["file-1", "file-2"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(store_body()))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let store = client
        .vector_stores()
        .create(
            VectorStoreCreateParams::new()
                .name("Support FAQ")
                .file_ids(vec!["file-1".into(), "file-2".into()]),
        )
        .await
        .unwrap();

    assert_eq!(store.id, "vs_123");
    assert_eq!(store.status, VectorStoreStatus::Completed);
    assert_eq!(store.file_counts.total, 2);
    assert_eq!(store.expires_after.unwrap().days, 7);
}

#[tokio::test]
async fn store_retrieve_sends_beta_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/vector_stores/vs_123"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(store_body()))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let store = client.vector_stores().retrieve("vs_123").await.unwrap();
    assert_eq!(store.name.as_deref(), Some("Support FAQ"));
}

#[tokio::test]
async fn store_list_sends_beta_header_and_returns_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/vector_stores"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [store_body()],
            "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let page = client.vector_stores().list(None).await.unwrap();
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "vs_123");
}

#[tokio::test]
async fn store_list_all_follows_cursor_with_beta_header_on_every_page() {
    let server = MockServer::start().await;
    // Page 2 — mounted first so the `after` match wins.
    Mock::given(method("GET"))
        .and(path("/v1/vector_stores"))
        .and(query_param("after", "vs_1"))
        .and(header("OpenAI-Beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "vs_2", "object": "vector_store", "created_at": 2}],
            "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    // Page 1.
    Mock::given(method("GET"))
        .and(path("/v1/vector_stores"))
        .and(header("OpenAI-Beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "vs_1", "object": "vector_store", "created_at": 1}],
            "has_more": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let stores = client.vector_stores().list_all().await.unwrap();
    assert_eq!(
        stores.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        vec!["vs_1", "vs_2"]
    );
}

#[tokio::test]
async fn store_delete_sends_beta_header() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/vector_stores/vs_123"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "vs_123", "object": "vector_store.deleted", "deleted": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let deleted = client.vector_stores().delete("vs_123").await.unwrap();
    assert!(deleted.deleted);
}

#[tokio::test]
async fn store_search_posts_query_and_returns_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/vector_stores/vs_123/search"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_partial_json(json!({
            "query": "how to reset password",
            "max_num_results": 3,
            "rewrite_query": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "vector_store.search_results.page",
            "data": [{
                "file_id": "file-1",
                "filename": "faq.md",
                "score": 0.87,
                "attributes": {"topic": "auth"},
                "content": [{"type": "text", "text": "Click reset password."}]
            }],
            "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let results = client
        .vector_stores()
        .search(
            "vs_123",
            VectorStoreSearchParams::new("how to reset password")
                .max_num_results(3)
                .rewrite_query(true),
        )
        .await
        .unwrap();

    assert_eq!(results.data.len(), 1);
    assert_eq!(results.data[0].file_id, "file-1");
    assert_eq!(results.data[0].content[0].text, "Click reset password.");
    assert!((results.data[0].score - 0.87).abs() < 1e-9);
}

#[tokio::test]
async fn search_accepts_multiple_query_strings() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/vector_stores/vs_123/search"))
        .and(body_partial_json(json!({ "query": ["a", "b"] })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "vector_store.search_results.page",
            "data": [],
            "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let results = client
        .vector_stores()
        .search(
            "vs_123",
            VectorStoreSearchParams::new(SearchQuery::Multiple(vec!["a".into(), "b".into()])),
        )
        .await
        .unwrap();
    assert!(results.data.is_empty());
}

#[tokio::test]
async fn file_create_sends_chunking_strategy_and_lists() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/vector_stores/vs_123/files"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_partial_json(json!({
            "file_id": "file-1",
            "chunking_strategy": {"type": "static", "static": {
                "max_chunk_size_tokens": 800, "chunk_overlap_tokens": 400
            }}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "file-1",
            "object": "vector_store.file",
            "created_at": 1700000001,
            "vector_store_id": "vs_123",
            "status": "in_progress",
            "usage_bytes": 0
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/vector_stores/vs_123/files"))
        .and(header("openai-beta", "assistants=v2"))
        .and(query_param("filter", "completed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{
                "id": "file-1", "object": "vector_store.file", "created_at": 1,
                "vector_store_id": "vs_123", "status": "completed", "usage_bytes": 10
            }],
            "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let files = client.vector_stores().files("vs_123");

    let created = files
        .create(VectorStoreFileCreateParams::new("file-1").chunking_strategy(json!({
            "type": "static",
            "static": {"max_chunk_size_tokens": 800, "chunk_overlap_tokens": 400}
        })))
        .await
        .unwrap();
    assert_eq!(created.status, VectorStoreFileStatus::InProgress);
    assert_eq!(created.vector_store_id, "vs_123");

    let page = files
        .list(Some(openai_compat::types::vector_stores::VectorStoreFileListParams {
            filter: Some("completed".into()),
            ..Default::default()
        }))
        .await
        .unwrap();
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].status, VectorStoreFileStatus::Completed);
}

#[tokio::test]
async fn file_batch_create_and_cancel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/vector_stores/vs_123/file_batches"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_partial_json(json!({ "file_ids": ["file-1", "file-2"] })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "vsfb_1",
            "object": "vector_store.files_batch",
            "created_at": 1700000002,
            "vector_store_id": "vs_123",
            "status": "in_progress",
            "file_counts": {
                "in_progress": 2, "completed": 0, "failed": 0, "cancelled": 0, "total": 2
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/vector_stores/vs_123/file_batches/vsfb_1/cancel"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "vsfb_1",
            "object": "vector_store.files_batch",
            "created_at": 1700000002,
            "vector_store_id": "vs_123",
            "status": "cancelled",
            "file_counts": {
                "in_progress": 0, "completed": 0, "failed": 0, "cancelled": 2, "total": 2
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let batches = client.vector_stores().file_batches("vs_123");

    let batch = batches
        .create(VectorStoreFileBatchCreateParams::new(vec![
            "file-1".into(),
            "file-2".into(),
        ]))
        .await
        .unwrap();
    assert_eq!(batch.id, "vsfb_1");
    assert_eq!(batch.file_counts.total, 2);

    let cancelled = batches.cancel("vsfb_1").await.unwrap();
    assert_eq!(cancelled.status, VectorStoreFileStatus::Cancelled);
    assert_eq!(cancelled.file_counts.cancelled, 2);
}
