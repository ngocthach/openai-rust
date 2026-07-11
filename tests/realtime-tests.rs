//! Integration tests for the realtime WebSocket module.
//!
//! Spins up a local `tokio-tungstenite` server that verifies the upgrade
//! headers, echoes a `session.created` event, and replies to a
//! `session.update` with `session.updated`. Plus unit-level tests for URL
//! building and event-constructor shapes.

use futures_util::{SinkExt, StreamExt};
use openai_compat::realtime::{self, build_realtime_url, events, RealtimeConnectOptions};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::Message;

/// Observed upgrade headers reported back by the test server.
type ObservedHeaders = (Option<String>, Option<String>);

/// Start a local realtime-style WebSocket server on an ephemeral port.
///
/// On connect it: verifies + reports the `Authorization` and `OpenAI-Beta`
/// headers, sends a `session.created` event, waits for a `session.update`
/// and replies with `session.updated`, then closes.
// The handshake callback's `Result` uses tungstenite's `ErrorResponse`, whose
// size is fixed by the `accept_hdr_async` API — the large-Err lint is moot here.
#[allow(clippy::result_large_err)]
async fn spawn_test_server(header_tx: oneshot::Sender<ObservedHeaders>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _peer) = listener.accept().await.unwrap();

        let callback =
            move |req: &Request, response: Response| -> Result<Response, ErrorResponse> {
                let header = |name: &str| {
                    req.headers()
                        .get(name)
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_owned)
                };
                let _ = header_tx.send((header("Authorization"), header("OpenAI-Beta")));
                Ok(response)
            };

        let mut ws = accept_hdr_async(stream, callback).await.unwrap();

        ws.send(Message::Text(
            json!({ "type": "session.created", "session": { "id": "sess_test" } }).to_string(),
        ))
        .await
        .unwrap();

        if let Some(Ok(Message::Text(text))) = ws.next().await {
            let received: Value = serde_json::from_str(text.as_str()).unwrap();
            assert_eq!(received["type"], "session.update");
            ws.send(Message::Text(
                json!({ "type": "session.updated", "session": received["session"].clone() })
                    .to_string(),
            ))
            .await
            .unwrap();
        } else {
            panic!("expected a session.update text frame");
        }

        ws.close(None).await.ok();
    });

    format!("ws://{addr}")
}

#[tokio::test]
async fn connect_send_recv_close_roundtrip() {
    let (header_tx, header_rx) = oneshot::channel();
    let url = spawn_test_server(header_tx).await;

    let mut session = realtime::connect(RealtimeConnectOptions {
        api_key: "sk-test-key".into(),
        base_url: url,
        model: "test-model".into(),
        organization: None,
        project: None,
        extra_headers: Vec::new(),
    })
    .await
    .expect("connection should succeed");

    // The server observed the upgrade headers we set.
    let (auth, beta) = header_rx.await.expect("server reported headers");
    assert_eq!(auth.as_deref(), Some("Bearer sk-test-key"));
    assert_eq!(beta.as_deref(), Some("realtime=v1"));

    // Server greets with session.created.
    let created = session
        .recv()
        .await
        .expect("recv ok")
        .expect("event present");
    assert_eq!(created["type"], "session.created");

    // Send a session.update; expect session.updated back.
    session
        .send(events::session_update(json!({ "modalities": ["text"] })))
        .await
        .expect("send ok");

    let updated = session
        .recv()
        .await
        .expect("recv ok")
        .expect("event present");
    assert_eq!(updated["type"], "session.updated");
    assert_eq!(updated["session"]["modalities"][0], "text");

    session.close().await.expect("close ok");
}

#[test]
fn url_builder_https_becomes_wss_with_model() {
    let url = build_realtime_url("https://api.openai.com/v1", "gpt-4o-realtime-preview").unwrap();
    assert_eq!(
        url,
        "wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview"
    );
}

#[test]
fn url_builder_http_becomes_ws() {
    let url = build_realtime_url("http://localhost:8080", "m").unwrap();
    assert_eq!(url, "ws://localhost:8080/realtime?model=m");
}

#[test]
fn url_builder_trims_trailing_slash() {
    let url = build_realtime_url("https://example.com/v1/", "m").unwrap();
    assert_eq!(url, "wss://example.com/v1/realtime?model=m");
}

#[test]
fn url_builder_defaults_empty_base_url() {
    let url = build_realtime_url("", "m").unwrap();
    assert_eq!(url, "wss://api.openai.com/v1/realtime?model=m");
}

#[test]
fn url_builder_accepts_ws_scheme_as_is() {
    let url = build_realtime_url("ws://127.0.0.1:9001", "m").unwrap();
    assert_eq!(url, "ws://127.0.0.1:9001/realtime?model=m");
}

#[test]
fn url_builder_percent_encodes_model() {
    let url = build_realtime_url("https://x.test", "a b/c").unwrap();
    assert_eq!(url, "wss://x.test/realtime?model=a%20b%2Fc");
}

#[test]
fn url_builder_rejects_unknown_scheme() {
    assert!(build_realtime_url("ftp://example.com", "m").is_err());
}

#[test]
fn event_session_update_shape() {
    let event = events::session_update(json!({ "voice": "alloy" }));
    assert_eq!(event["type"], "session.update");
    assert_eq!(event["session"]["voice"], "alloy");
}

#[test]
fn event_input_audio_buffer_append_shape() {
    let event = events::input_audio_buffer_append("QUJD");
    assert_eq!(event["type"], "input_audio_buffer.append");
    assert_eq!(event["audio"], "QUJD");
}

#[test]
fn event_input_audio_buffer_commit_shape() {
    let event = events::input_audio_buffer_commit();
    assert_eq!(event, json!({ "type": "input_audio_buffer.commit" }));
}

#[test]
fn event_conversation_item_create_shape() {
    let item = json!({ "type": "message", "role": "user" });
    let event = events::conversation_item_create(item.clone());
    assert_eq!(event["type"], "conversation.item.create");
    assert_eq!(event["item"], item);
}

#[test]
fn event_response_create_shape() {
    assert_eq!(
        events::response_create(),
        json!({ "type": "response.create" })
    );
}

#[test]
fn event_response_create_with_shape() {
    let event = events::response_create_with(json!({ "instructions": "be brief" }));
    assert_eq!(event["type"], "response.create");
    assert_eq!(event["response"]["instructions"], "be brief");
}
