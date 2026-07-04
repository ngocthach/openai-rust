//! Integration tests for the Assistants (beta v2) resources against a local
//! wiremock server. Every test asserts the `OpenAI-Beta: assistants=v2` header
//! is sent, plus body/response shape for the key endpoints.

use openai_compat::types::assistants::{
    AssistantCreateParams, MessageCreateParams, RunCreateParams, RunStatus,
    SubmitToolOutputsParams, ThreadCreateParams, ToolOutput,
};
use openai_compat::Client;
use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-test-key")
        .base_url(format!("{}/v1", server.uri()))
        .max_retries(2)
        .build()
        .unwrap()
}

#[tokio::test]
async fn assistant_create_sends_beta_header_and_parses() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/assistants"))
        .and(header("openai-beta", "assistants=v2"))
        .and(header("authorization", "Bearer sk-test-key"))
        .and(body_partial_json(json!({
            "model": "gpt-4o",
            "name": "Math Tutor",
            "instructions": "You are a math tutor."
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "asst_123",
            "object": "assistant",
            "created_at": 1710000000i64,
            "model": "gpt-4o",
            "name": "Math Tutor",
            "instructions": "You are a math tutor.",
            "tools": [{"type": "code_interpreter"}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let assistant = client
        .assistants()
        .create(
            AssistantCreateParams::new("gpt-4o")
                .name("Math Tutor")
                .instructions("You are a math tutor."),
        )
        .await
        .unwrap();

    assert_eq!(assistant.id, "asst_123");
    assert_eq!(assistant.name.as_deref(), Some("Math Tutor"));
    assert_eq!(assistant.tools.len(), 1);
}

#[tokio::test]
async fn thread_message_run_flow_and_status_parsing() {
    let server = MockServer::start().await;

    // Thread create.
    Mock::given(method("POST"))
        .and(path("/v1/threads"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "thread_abc",
            "object": "thread",
            "created_at": 1710000000i64
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Message create.
    Mock::given(method("POST"))
        .and(path("/v1/threads/thread_abc/messages"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_partial_json(json!({
            "role": "user",
            "content": "What is 2 + 2?"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_1",
            "object": "thread.message",
            "created_at": 1710000001i64,
            "thread_id": "thread_abc",
            "role": "user",
            "status": "completed",
            "content": [
                {"type": "text", "text": {"value": "What is 2 + 2?", "annotations": []}}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Run create.
    Mock::given(method("POST"))
        .and(path("/v1/threads/thread_abc/runs"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_partial_json(json!({"assistant_id": "asst_123"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "run_1",
            "object": "thread.run",
            "created_at": 1710000002i64,
            "thread_id": "thread_abc",
            "assistant_id": "asst_123",
            "status": "queued",
            "model": "gpt-4o",
            "parallel_tool_calls": true,
            "tools": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Run retrieve -> completed with usage.
    Mock::given(method("GET"))
        .and(path("/v1/threads/thread_abc/runs/run_1"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "run_1",
            "object": "thread.run",
            "created_at": 1710000002i64,
            "thread_id": "thread_abc",
            "assistant_id": "asst_123",
            "status": "completed",
            "model": "gpt-4o",
            "parallel_tool_calls": true,
            "tools": [],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);

    let thread = client
        .threads()
        .create(ThreadCreateParams::new())
        .await
        .unwrap();
    assert_eq!(thread.id, "thread_abc");

    let message = client
        .threads()
        .messages()
        .create(&thread.id, MessageCreateParams::user("What is 2 + 2?"))
        .await
        .unwrap();
    assert_eq!(message.text().as_deref(), Some("What is 2 + 2?"));

    let run = client
        .threads()
        .runs()
        .create(&thread.id, RunCreateParams::new("asst_123"))
        .await
        .unwrap();
    assert_eq!(run.status, RunStatus::Queued);

    let run = client
        .threads()
        .runs()
        .retrieve(&thread.id, &run.id)
        .await
        .unwrap();
    assert_eq!(run.status, RunStatus::Completed);
    assert_eq!(run.usage.unwrap().total_tokens, 15);
}

#[tokio::test]
async fn submit_tool_outputs_sends_body_and_beta_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/threads/thread_abc/runs/run_1/submit_tool_outputs"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_partial_json(json!({
            "tool_outputs": [
                {"tool_call_id": "call_1", "output": "42"}
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "run_1",
            "object": "thread.run",
            "created_at": 1710000002i64,
            "thread_id": "thread_abc",
            "assistant_id": "asst_123",
            "status": "in_progress",
            "model": "gpt-4o",
            "parallel_tool_calls": true,
            "tools": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let run = client
        .threads()
        .runs()
        .submit_tool_outputs(
            "thread_abc",
            "run_1",
            SubmitToolOutputsParams::new(vec![ToolOutput::new("call_1", "42")]),
        )
        .await
        .unwrap();

    assert_eq!(run.status, RunStatus::InProgress);
}

#[tokio::test]
async fn run_steps_list_sends_beta_header_and_parses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/threads/thread_abc/runs/run_1/steps"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [
                {
                    "id": "step_1",
                    "object": "thread.run.step",
                    "created_at": 1710000003i64,
                    "run_id": "run_1",
                    "thread_id": "thread_abc",
                    "assistant_id": "asst_123",
                    "type": "message_creation",
                    "status": "completed",
                    "step_details": {
                        "type": "message_creation",
                        "message_creation": {"message_id": "msg_1"}
                    }
                }
            ],
            "has_more": false,
            "first_id": "step_1",
            "last_id": "step_1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let page = client
        .threads()
        .runs()
        .steps()
        .list("thread_abc", "run_1", None)
        .await
        .unwrap();

    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "step_1");
    assert_eq!(page.data[0].step_type, "message_creation");
    assert_eq!(page.data[0].status, "completed");
}
