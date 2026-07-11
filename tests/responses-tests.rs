//! Integration tests for the Responses API resource against a local wiremock
//! server: create/retrieve/delete/cancel, streaming (happy path, unknown
//! event, failed-is-typed), and input_items pagination.

use futures_util::StreamExt;
use openai_compat::types::responses::ResponseStreamEvent;
use openai_compat::{Client, CreateResponseRequest};
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

fn response_body(id: &str, status: &str) -> serde_json::Value {
    json!({
        "id": id,
        "object": "response",
        "created_at": 1.0,
        "status": status,
        "model": "gpt-5",
        "output": [{
            "type": "message",
            "id": "msg_1",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "Hello there!", "annotations": []}]
        }],
        "usage": {
            "input_tokens": 10, "output_tokens": 5, "total_tokens": 15,
            "input_tokens_details": {"cached_tokens": 0},
            "output_tokens_details": {"reasoning_tokens": 0}
        }
    })
}

#[tokio::test]
async fn create_non_streaming() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(body_partial_json(json!({"model": "gpt-5", "input": "hi"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(response_body("resp_1", "completed")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let response = client
        .responses()
        .create(CreateResponseRequest::new("gpt-5", "hi"))
        .await
        .unwrap();

    assert_eq!(response.id, "resp_1");
    assert_eq!(response.output_text(), "Hello there!");
    assert_eq!(response.usage.unwrap().total_tokens, 15);
}

#[tokio::test]
async fn create_with_function_tool_and_output() {
    use openai_compat::types::responses::Tool;

    let server = MockServer::start().await;
    let mut body = response_body("resp_2", "completed");
    body["output"] = json!([{
        "type": "function_call",
        "id": "fc_1",
        "call_id": "call_1",
        "name": "get_weather",
        "arguments": "{\"city\":\"Hanoi\"}"
    }]);
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(body_partial_json(
            json!({"tools": [{"type": "function", "name": "get_weather"}]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let request =
        CreateResponseRequest::new("gpt-5", "weather in Hanoi?").tools(vec![Tool::function(
            "get_weather",
            "Get current weather",
            json!({"type": "object"}),
        )]);
    let response = client.responses().create(request).await.unwrap();
    let calls = response.function_calls();
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn create_streaming_ordered_events() {
    let sse_body = concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"created_at\":1.0,\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[]},\"sequence_number\":1}\n\n",
        "data: {\"type\":\"response.in_progress\",\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"created_at\":1.0,\"model\":\"gpt-5\",\"status\":\"in_progress\",\"output\":[]},\"sequence_number\":2}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hel\",\"sequence_number\":3}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"lo\",\"sequence_number\":4}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"created_at\":1.0,\"model\":\"gpt-5\",\"status\":\"completed\",\"output\":[]},\"sequence_number\":5}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
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
        .responses()
        .create_stream(CreateResponseRequest::new("gpt-5", "hi"))
        .await
        .unwrap();

    let mut text = String::new();
    let mut events = 0;
    let mut saw_completed = false;
    let mut last_sequence_number = None;
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        events += 1;
        last_sequence_number = event.sequence_number();
        match event {
            ResponseStreamEvent::OutputTextDelta { delta, .. } => text.push_str(&delta),
            ResponseStreamEvent::Completed { .. } => saw_completed = true,
            _ => {}
        }
    }
    assert_eq!(events, 5);
    assert_eq!(text, "Hello");
    assert!(saw_completed);
    assert_eq!(
        last_sequence_number,
        Some(5),
        "resumable streaming needs the last sequence_number"
    );
}

#[tokio::test]
async fn stream_unknown_event_tolerated() {
    let sse_body = concat!(
        "data: {\"type\":\"response.reasoning_summary_text.delta\",\"item_id\":\"r_1\",\"delta\":\"thinking\"}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"hi\",\"sequence_number\":1}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut stream = client
        .responses()
        .create_stream(CreateResponseRequest::new("gpt-5", "hi"))
        .await
        .unwrap();

    let first = stream.next().await.unwrap().unwrap();
    assert!(matches!(first, ResponseStreamEvent::Unknown));
    let second = stream.next().await.unwrap().unwrap();
    assert!(matches!(
        second,
        ResponseStreamEvent::OutputTextDelta { .. }
    ));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn stream_failed_is_typed_ok_not_stream_error() {
    let sse_body = "data: {\"type\":\"response.failed\",\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"created_at\":1.0,\"model\":\"gpt-5\",\"status\":\"failed\",\"output\":[],\"error\":{\"code\":\"server_error\",\"message\":\"boom\"}},\"sequence_number\":1}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut stream = client
        .responses()
        .create_stream(CreateResponseRequest::new("gpt-5", "hi"))
        .await
        .unwrap();

    let event = stream.next().await.unwrap().unwrap();
    match event {
        ResponseStreamEvent::Failed { response, .. } => {
            assert_eq!(response.error.unwrap().code, "server_error");
        }
        other => panic!("expected typed Ok(Failed), got {other:?}"),
    }
}

#[tokio::test]
async fn retrieve_stream_resumes_background_response_after_cursor() {
    let sse_body = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"lo\",\"sequence_number\":4}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"created_at\":1.0,\"model\":\"gpt-5\",\"status\":\"completed\",\"output\":[]},\"sequence_number\":5}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/responses/resp_1"))
        .and(query_param("stream", "true"))
        .and(query_param("starting_after", "3"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(sse_body, "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut stream = client
        .responses()
        .retrieve_stream("resp_1", Some(3))
        .await
        .unwrap();

    let mut last_sequence_number = None;
    let mut saw_completed = false;
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        last_sequence_number = event.sequence_number();
        if matches!(event, ResponseStreamEvent::Completed { .. }) {
            saw_completed = true;
        }
    }
    assert!(saw_completed);
    assert_eq!(last_sequence_number, Some(5));
}

#[tokio::test]
async fn retrieve_sends_include_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/responses/resp_1"))
        .and(query_param("include[]", "message.output_text.logprobs"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(response_body("resp_1", "completed")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = openai_compat::resources::responses::RetrieveParams::new().include(vec![
        openai_compat::types::responses::ResponseIncludable::MessageOutputTextLogprobs,
    ]);
    let response = client
        .responses()
        .retrieve("resp_1", Some(params))
        .await
        .unwrap();
    assert_eq!(
        response.status,
        Some(openai_compat::types::responses::ResponseStatus::Completed)
    );
}

#[tokio::test]
async fn delete_handles_empty_body() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/responses/resp_1"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    client.responses().delete("resp_1").await.unwrap();
}

#[tokio::test]
async fn cancel_returns_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses/resp_1/cancel"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(response_body("resp_1", "cancelled")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let response = client.responses().cancel("resp_1").await.unwrap();
    assert_eq!(
        response.status,
        Some(openai_compat::types::responses::ResponseStatus::Cancelled)
    );
}

#[tokio::test]
async fn input_items_pagination_follows_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/responses/resp_1/input_items"))
        .and(query_param("limit", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "item_1", "type": "message"}],
            "has_more": true,
            "last_id": "item_1"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/responses/resp_1/input_items"))
        .and(query_param("after", "item_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "item_2", "type": "message"}],
            "has_more": false
        })))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = openai_compat::resources::responses::InputItemsListParams::new().limit(1);
    let items = client
        .responses()
        .input_items("resp_1")
        .list_all(Some(params))
        .await
        .unwrap();
    assert_eq!(items.len(), 2);
}
