//! Integration tests against a local wiremock server: request shape, error
//! mapping, retry behavior, streaming, and pagination.

use futures_util::StreamExt;
use openai_compat::types::embeddings::{EmbeddingRequest, EmbeddingVector};
use openai_compat::types::files::FileUpload;
use openai_compat::{
    ApiErrorKind, ChatCompletionRequest, Client, Message, OpenAIError, ToolChoice,
};
use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn client_for(server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-test-key")
        .base_url(format!("{}/v1", server.uri()))
        .organization("org-test")
        .project("proj-test")
        .max_retries(2)
        .build()
        .unwrap()
}

fn chat_response_body() -> serde_json::Value {
    json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "created": 1728933352,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Hello!"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}
    })
}

#[tokio::test]
async fn chat_completion_sends_auth_and_parses_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-test-key"))
        .and(header("openai-organization", "org-test"))
        .and(header("openai-project", "proj-test"))
        .and(body_partial_json(json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Say hello"}],
            "temperature": 0.5
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_response_body()))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let completion = client
        .chat()
        .completions()
        .create(
            ChatCompletionRequest::new("gpt-4o", vec![Message::user("Say hello")]).temperature(0.5),
        )
        .await
        .unwrap();

    assert_eq!(completion.content(), Some("Hello!"));
    assert_eq!(completion.usage.unwrap().total_tokens, 7);
}

#[tokio::test]
async fn non_retryable_error_is_mapped_and_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("x-request-id", "req_abc")
                .set_body_json(json!({
                    "error": {
                        "message": "Incorrect API key provided",
                        "type": "invalid_request_error",
                        "code": "invalid_api_key"
                    }
                })),
        )
        .expect(1) // exactly one attempt: 401 must not be retried
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap_err();

    let OpenAIError::Api(api) = err else {
        panic!("expected Api error, got {err:?}");
    };
    assert_eq!(api.status, 401);
    assert_eq!(api.kind, ApiErrorKind::Authentication);
    assert_eq!(api.request_id.as_deref(), Some("req_abc"));
    assert_eq!(api.detail.unwrap().code.as_deref(), Some("invalid_api_key"));
}

/// Responds 500 twice, then 200 — verifies the retry loop recovers.
struct FlakyResponder {
    failures: std::sync::atomic::AtomicU32,
}

impl Respond for FlakyResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let n = self
            .failures
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n < 2 {
            // retry-after-ms keeps the test fast (no exponential backoff wait).
            ResponseTemplate::new(500)
                .insert_header("retry-after-ms", "5")
                .set_body_json(json!({"error": {"message": "server exploded"}}))
        } else {
            ResponseTemplate::new(200).set_body_json(chat_response_body())
        }
    }
}

#[tokio::test]
async fn retryable_error_is_retried_until_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(FlakyResponder {
            failures: std::sync::atomic::AtomicU32::new(0),
        })
        .expect(3) // 2 failures + 1 success with max_retries=2
        .mount(&server)
        .await;

    let client = client_for(&server);
    let completion = client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap();
    assert_eq!(completion.content(), Some("Hello!"));
}

#[tokio::test]
async fn retries_exhausted_surfaces_last_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after-ms", "5")
                .set_body_json(json!({"error": {"message": "rate limited"}})),
        )
        .expect(3) // initial + 2 retries
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap_err();

    let OpenAIError::Api(api) = err else {
        panic!("expected Api error");
    };
    assert_eq!(api.kind, ApiErrorKind::RateLimit);
}

#[tokio::test]
async fn x_should_retry_false_prevents_retry_of_500() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(500)
                .insert_header("x-should-retry", "false")
                .set_body_string("nope"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client
        .chat()
        .completions()
        .create(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap_err();
    assert!(matches!(err, OpenAIError::Api(api) if api.status == 500));
}

#[tokio::test]
async fn chat_streaming_yields_chunks_and_stops_at_done() {
    let sse_body = concat!(
        "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(json!({"stream": true})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(sse_body, "text/event-stream"),
        )
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut stream = client
        .chat()
        .completions()
        .create_stream(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap();

    let mut text = String::new();
    let mut chunks = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        if let Some(content) = chunk.content() {
            text.push_str(content);
        }
        chunks += 1;
    }
    assert_eq!(chunks, 3);
    assert_eq!(text, "Hello");
}

#[tokio::test]
async fn stream_error_event_surfaces_as_error() {
    let sse_body = "data: {\"error\": {\"message\": \"boom mid-stream\"}}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut stream = client
        .chat()
        .completions()
        .create_stream(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap();

    let item = stream.next().await.unwrap();
    let err = item.unwrap_err();
    assert!(matches!(err, OpenAIError::Stream(msg) if msg.contains("boom mid-stream")));
    assert!(stream.next().await.is_none(), "stream ends after error");
}

#[tokio::test]
async fn truncated_stream_without_done_terminates_cleanly() {
    // Stream ends without `data: [DONE]` and without a final newline: the
    // decoder tail must be flushed and the stream must terminate (not
    // re-poll the exhausted body).
    let sse_body = concat!(
        "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"c1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"tail\"},\"finish_reason\":null}]}",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut stream = client
        .chat()
        .completions()
        .create_stream(ChatCompletionRequest::new(
            "gpt-4o",
            vec![Message::user("hi")],
        ))
        .await
        .unwrap();

    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        if let Some(content) = chunk.unwrap().content() {
            text.push_str(content);
        }
    }
    assert_eq!(text, "partialtail");
    // Polling again after completion must stay terminated.
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn embeddings_and_models_and_delete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .and(body_partial_json(json!({
            "model": "text-embedding-3-small",
            "input": "hello",
            "dimensions": 4
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"object": "embedding", "index": 0, "embedding": [0.1, 0.2, 0.3, 0.4]}],
            "model": "text-embedding-3-small",
            "usage": {"prompt_tokens": 1, "total_tokens": 1}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [
                {"id": "gpt-4o", "object": "model", "created": 1, "owned_by": "openai"},
                {"id": "gpt-4o-mini", "object": "model", "created": 2, "owned_by": "openai"}
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1/models/ft-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ft-1", "object": "model", "deleted": true
        })))
        .mount(&server)
        .await;

    let client = client_for(&server);

    let embeddings = client
        .embeddings()
        .create(EmbeddingRequest::new("text-embedding-3-small", "hello").dimensions(4))
        .await
        .unwrap();
    let EmbeddingVector::Floats(vector) = &embeddings.data[0].embedding else {
        panic!("expected float vector");
    };
    assert_eq!(vector.len(), 4);

    let models = client.models().list().await.unwrap();
    assert_eq!(models.data.len(), 2);
    assert_eq!(models.data[0].id, "gpt-4o");

    let deleted = client.models().delete("ft-1").await.unwrap();
    assert!(deleted.deleted);
}

#[tokio::test]
async fn files_pagination_follows_after_cursor() {
    let server = MockServer::start().await;
    // Page 2 (has explicit `after` param) — mount first so it wins the match.
    Mock::given(method("GET"))
        .and(path("/v1/files"))
        .and(query_param("after", "file-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "file-3", "object": "file", "filename": "c.txt"}],
            "has_more": false
        })))
        .mount(&server)
        .await;
    // Page 1.
    Mock::given(method("GET"))
        .and(path("/v1/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [
                {"id": "file-1", "object": "file", "filename": "a.txt"},
                {"id": "file-2", "object": "file", "filename": "b.txt"}
            ],
            "has_more": true
        })))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let files = client.files().list_all(None).await.unwrap();
    assert_eq!(
        files.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
        vec!["file-1", "file-2", "file-3"]
    );
}

#[tokio::test]
async fn file_upload_sends_multipart_and_content_returns_bytes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "file-9", "object": "file", "filename": "data.jsonl",
            "purpose": "fine-tune", "bytes": 12, "created_at": 1
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/files/file-9/content"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"raw-content".to_vec()))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let file = client
        .files()
        .create(
            FileUpload::new("data.jsonl", b"{\"a\":1}\n".to_vec()),
            "fine-tune",
        )
        .await
        .unwrap();
    assert_eq!(file.id, "file-9");

    let content = client.files().content("file-9").await.unwrap();
    assert_eq!(&content[..], b"raw-content");

    // Verify the upload was proper multipart/form-data with both fields.
    let requests = server.received_requests().await.unwrap();
    let upload = requests
        .iter()
        .find(|r| r.url.path() == "/v1/files")
        .unwrap();
    let content_type = upload
        .headers
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.starts_with("multipart/form-data"));
    let body = String::from_utf8_lossy(&upload.body);
    assert!(body.contains("name=\"file\""));
    assert!(body.contains("filename=\"data.jsonl\""));
    assert!(body.contains("name=\"purpose\""));
    assert!(body.contains("fine-tune"));
}

#[tokio::test]
async fn audio_speech_returns_binary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/speech"))
        .and(body_partial_json(json!({
            "model": "tts-1", "input": "hello", "voice": "alloy"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0x49, 0x44, 0x33, 0x04]))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let audio = client
        .audio()
        .speech(openai_compat::types::audio::SpeechRequest::new(
            "tts-1", "hello", "alloy",
        ))
        .await
        .unwrap();
    assert_eq!(&audio[..2], &[0x49, 0x44]);
}

#[tokio::test]
async fn tool_call_round_trip() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(json!({
            "tools": [{
                "type": "function",
                "function": {"name": "get_weather", "description": "Get weather"}
            }],
            "tool_choice": "auto"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-1", "object": "chat.completion", "created": 1, "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1", "type": "function",
                        "function": {"name": "get_weather", "arguments": "{\"city\":\"Hanoi\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        })))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let request = ChatCompletionRequest::new("gpt-4o", vec![Message::user("Weather in Hanoi?")])
        .tools(vec![openai_compat::Tool::function(
            "get_weather",
            "Get weather",
            json!({"type": "object", "properties": {"city": {"type": "string"}}}),
        )])
        .tool_choice(ToolChoice::Auto);
    let completion = client.chat().completions().create(request).await.unwrap();
    let calls = completion.choices[0].message.tool_calls.as_ref().unwrap();
    assert_eq!(calls[0].function.name, "get_weather");
}

#[tokio::test]
async fn generic_escape_hatch_get_and_post() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/some/custom/endpoint"))
        .and(query_param("limit", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/another/endpoint"))
        .and(body_partial_json(json!({"custom": "payload"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"created": "yes"})))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let value: serde_json::Value = client
        .get("/some/custom/endpoint", &[("limit", "5")])
        .await
        .unwrap();
    assert_eq!(value["ok"], true);

    let value: serde_json::Value = client
        .post("/another/endpoint", &json!({"custom": "payload"}))
        .await
        .unwrap();
    assert_eq!(value["created"], "yes");
}
