//! Integration tests for the Azure OpenAI client mode: base URL shape,
//! `api-version` query, `api-key`/AD-token auth, and deployment path
//! rewriting — mirroring `openai-python/src/openai/lib/azure.py`.

use openai_compat::types::audio::TranscriptionRequest;
use openai_compat::types::files::FileUpload;
use openai_compat::{ChatCompletionRequest, Client, Message};
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn chat_body() -> serde_json::Value {
    json!({
        "id": "chatcmpl-1", "object": "chat.completion", "created": 1, "model": "gpt-4o",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"}, "finish_reason": "stop"}]
    })
}

#[tokio::test]
async fn azure_with_deployment_uses_api_key_and_api_version() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/my-gpt4o/chat/completions"))
        .and(query_param("api-version", "2024-06-01"))
        .and(header("api-key", "azure-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_body()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("azure-key")
        .azure(server.uri(), "2024-06-01")
        .azure_deployment("my-gpt4o")
        .build()
        .unwrap();

    let completion = client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap();
    assert_eq!(completion.content(), Some("hi"));

    // api-key auth must not also send a Bearer Authorization header.
    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].headers.get("authorization").is_none());
}

#[tokio::test]
async fn azure_without_deployment_derives_path_from_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/gpt-4o-mini/chat/completions"))
        .and(query_param("api-version", "2024-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_body()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("azure-key")
        .azure(server.uri(), "2024-06-01")
        .build()
        .unwrap();

    client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new(
            "gpt-4o-mini",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap();
}

#[tokio::test]
async fn azure_non_deployment_endpoint_is_not_rewritten() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/openai/models"))
        .and(query_param("api-version", "2024-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "gpt-4o", "object": "model"}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("azure-key")
        .azure(server.uri(), "2024-06-01")
        .build()
        .unwrap();
    let models = client.models().list().await.unwrap();
    assert_eq!(models.data.len(), 1);
}

#[tokio::test]
async fn azure_pinned_deployment_leaves_non_deployment_endpoints_alone() {
    // Regression: with a pinned deployment, /models must NOT gain a
    // /deployments/{dep} segment (mirrors azure.py _prepare_url).
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/openai/models"))
        .and(query_param("api-version", "2024-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "gpt-4o", "object": "model"}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("azure-key")
        .azure(server.uri(), "2024-06-01")
        .azure_deployment("my-gpt4o")
        .build()
        .unwrap();
    let models = client.models().list().await.unwrap();
    assert_eq!(models.data[0].id, "gpt-4o");
}

#[test]
fn builder_debug_redacts_secrets() {
    let builder = Client::builder()
        .api_key("sk-super-secret")
        .azure("https://example.openai.azure.com", "2024-06-01")
        .azure_ad_token("ad-token-secret");
    let debug = format!("{builder:?}");
    assert!(!debug.contains("sk-super-secret"));
    assert!(!debug.contains("ad-token-secret"));
    assert!(debug.contains("[REDACTED]"));
}

#[tokio::test]
async fn azure_ad_token_uses_bearer_authorization() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/dep/chat/completions"))
        .and(header("authorization", "Bearer ad-token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_body()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .azure(server.uri(), "2024-06-01")
        .azure_deployment("dep")
        .azure_ad_token("ad-token-123")
        .build()
        .unwrap();

    client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].headers.get("api-key").is_none());
    assert!(requests[0].headers.get("authorization").is_some());
}

#[tokio::test]
async fn azure_requires_api_version_and_credentials() {
    // Missing api_version (and no OPENAI_API_VERSION env assumed in CI).
    if std::env::var("OPENAI_API_VERSION").is_err() {
        let err = Client::builder()
            .api_key("k")
            .azure("https://example.openai.azure.com", "")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("api_version"), "got: {err}");
    }

    // Missing credentials entirely.
    if std::env::var("AZURE_OPENAI_API_KEY").is_err()
        && std::env::var("AZURE_OPENAI_AD_TOKEN").is_err()
        && std::env::var("OPENAI_API_KEY").is_err()
    {
        let err = Client::builder()
            .azure("https://example.openai.azure.com", "2024-06-01")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("credentials"), "got: {err}");
    }
}

#[tokio::test]
async fn azure_pinned_deployment_does_not_rewrite_body_less_requests() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/openai/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("k")
        .azure(server.uri(), "2024-06-01")
        .azure_deployment("my-gpt4o")
        .build()
        .unwrap();
    let _: serde_json::Value = client.get("/chat/completions", &[]).await.unwrap();
}

#[tokio::test]
async fn azure_pinned_deployment_does_not_rewrite_stored_chat_completion_list() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/openai/chat/completions"))
        .and(query_param("api-version", "2024-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": [], "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("k")
        .azure(server.uri(), "2024-06-01")
        .azure_deployment("my-gpt4o")
        .build()
        .unwrap();
    let page = client.chat().completions().list(None).await.unwrap();
    assert!(page.data.is_empty());
}

#[tokio::test]
async fn azure_pinned_deployment_routes_multipart_requests() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/whisper/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"text": "hi"})))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("k")
        .azure(server.uri(), "2024-06-01")
        .azure_deployment("whisper")
        .build()
        .unwrap();
    let transcription = client
        .audio()
        .transcriptions(
            FileUpload::new("a.wav", b"RIFF".to_vec()),
            TranscriptionRequest::new("whisper-1"),
        )
        .await
        .unwrap();
    assert_eq!(transcription.text, "hi");
}

#[tokio::test]
async fn azure_derived_deployment_is_percent_encoded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_body()))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("k")
        .azure(server.uri(), "2024-06-01")
        .build()
        .unwrap();
    client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new(
            "org/model",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].url.path(),
        "/openai/deployments/org%2Fmodel/chat/completions"
    );

    let err = client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new("..", vec![Message::user("hi")]))
        .await
        .unwrap_err();
    assert!(matches!(err, openai_compat::OpenAIError::Config(_)));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[test]
fn azure_both_explicit_credentials_is_config_error() {
    let err = Client::builder()
        .api_key("k")
        .azure_ad_token("t")
        .azure("https://example.openai.azure.com", "2024-06-01")
        .build()
        .unwrap_err();
    assert!(matches!(err, openai_compat::OpenAIError::Config(_)));
}

#[test]
fn azure_explicit_blank_credential_is_config_error() {
    let err = Client::builder()
        .api_key("")
        .azure("https://example.openai.azure.com", "2024-06-01")
        .build()
        .unwrap_err();
    assert!(matches!(err, openai_compat::OpenAIError::Config(_)));
}
