//! Realtime WebSocket support for the OpenAI Realtime API.
//!
//! This module provides a thin async session over a WebSocket connection:
//! it opens a connection (mirroring `openai-python`'s
//! `resources/realtime/realtime.py` URL/header building), then lets you
//! [`RealtimeSession::send`] and [`RealtimeSession::recv`] JSON events.
//!
//! # Scope
//!
//! The full typed event surface of the Realtime API is large (80+ client and
//! server event types). Typing every one of them is intentionally **out of
//! scope** here. Instead, every event is a [`serde_json::Value`] carrying the
//! standard envelope `{"type": "...", ...}`. A handful of typed constructors
//! for the most common client events live in the [`events`] submodule; build
//! any other event as a plain JSON value.
//!
//! ```no_run
//! use openai_compat::realtime::{self, RealtimeConnectOptions};
//! use serde_json::json;
//!
//! # async fn run() -> Result<(), realtime::RealtimeError> {
//! let mut session = realtime::connect(RealtimeConnectOptions {
//!     api_key: "sk-...".into(),
//!     base_url: "https://api.openai.com/v1".into(),
//!     model: "gpt-4o-realtime-preview".into(),
//!     organization: None,
//!     project: None,
//!     extra_headers: Vec::new(),
//! })
//! .await?;
//!
//! session
//!     .send(realtime::events::session_update(json!({ "modalities": ["text"] })))
//!     .await?;
//!
//! while let Some(event) = session.recv().await? {
//!     if event["type"] == "response.done" {
//!         break;
//!     }
//! }
//! session.close().await?;
//! # Ok(())
//! # }
//! ```

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

/// Errors produced by the realtime WebSocket module.
///
/// Kept local to this module (rather than reusing the crate-wide
/// `OpenAIError`) so the realtime surface stays self-contained.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RealtimeError {
    /// Failed to build the request or establish the WebSocket connection.
    #[error("realtime connection error: {0}")]
    Connect(String),
    /// A WebSocket protocol-level failure while sending or receiving frames.
    #[error("realtime protocol error: {0}")]
    Protocol(String),
    /// Failed to (de)serialize a JSON event.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Options controlling how a realtime connection is opened.
///
/// `connect` takes `base_url` + `api_key` directly (rather than a `Client`)
/// so the realtime module has no dependency on client configuration; an
/// accessor on the client can populate these fields.
#[derive(Debug, Clone)]
pub struct RealtimeConnectOptions {
    /// API key sent as `Authorization: Bearer <api_key>`.
    pub api_key: String,
    /// HTTP(S) base URL, e.g. `https://api.openai.com/v1`. The scheme is
    /// converted to `ws`/`wss` and `/realtime` is appended.
    pub base_url: String,
    /// Realtime model, sent as the `model` query parameter.
    pub model: String,
    /// Optional `OpenAI-Organization` header.
    pub organization: Option<String>,
    /// Optional `OpenAI-Project` header.
    pub project: Option<String>,
    /// Additional headers to attach to the upgrade request.
    pub extra_headers: Vec<(String, String)>,
}

/// Build the realtime WebSocket URL from an HTTP(S) base URL.
///
/// Mirrors `_prepare_url` in `realtime.py`: swap the scheme (`http`→`ws`,
/// `https`→`wss`), strip a trailing slash, append `/realtime`, and add the
/// `model` query parameter. An empty `base_url` falls back to the crate
/// default. A `ws://`/`wss://` base is accepted as-is (useful for tests).
pub fn build_realtime_url(base_url: &str, model: &str) -> Result<String, RealtimeError> {
    let base = base_url.trim();
    let base = if base.is_empty() {
        crate::DEFAULT_BASE_URL
    } else {
        base
    };
    let base = base.trim_end_matches('/');

    // A query or fragment on the base URL would end up in the middle of the
    // final URL (`.../v1?k=v/realtime?model=...`) — reject rather than emit
    // a silently broken URL.
    if base.contains('?') || base.contains('#') {
        return Err(RealtimeError::Connect(format!(
            "base_url must not contain a query or fragment: {base}"
        )));
    }

    let ws_base = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if base.starts_with("wss://") || base.starts_with("ws://") {
        base.to_string()
    } else {
        return Err(RealtimeError::Connect(format!(
            "unsupported base_url scheme: {base}"
        )));
    };

    Ok(format!(
        "{ws_base}/realtime?model={}",
        encode_query_component(model)
    ))
}

/// Minimal percent-encoding for a query-parameter value. Realtime model
/// names are simple, but this keeps the URL well-formed if one contains a
/// reserved character.
fn encode_query_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Open a realtime WebSocket connection.
///
/// Sets `Authorization: Bearer <api_key>` and `OpenAI-Beta: realtime=v1`,
/// plus org/project and any `extra_headers`, then connects over TLS (rustls).
pub async fn connect(options: RealtimeConnectOptions) -> Result<RealtimeSession, RealtimeError> {
    let url = build_realtime_url(&options.base_url, &options.model)?;

    let mut request = url
        .into_client_request()
        .map_err(|e| RealtimeError::Connect(e.to_string()))?;
    let headers = request.headers_mut();

    let auth = HeaderValue::from_str(&format!("Bearer {}", options.api_key))
        .map_err(|e| RealtimeError::Connect(format!("invalid api key header: {e}")))?;
    headers.insert("Authorization", auth);
    // Beta-era header; the GA API ignores it but some OpenAI-compatible
    // gateways still expect it, so it is kept for compatibility.
    headers.insert("OpenAI-Beta", HeaderValue::from_static("realtime=v1"));

    if let Some(org) = &options.organization {
        let value = HeaderValue::from_str(org)
            .map_err(|e| RealtimeError::Connect(format!("invalid organization header: {e}")))?;
        headers.insert("OpenAI-Organization", value);
    }
    if let Some(project) = &options.project {
        let value = HeaderValue::from_str(project)
            .map_err(|e| RealtimeError::Connect(format!("invalid project header: {e}")))?;
        headers.insert("OpenAI-Project", value);
    }
    for (name, value) in &options.extra_headers {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| RealtimeError::Connect(format!("invalid header name {name}: {e}")))?;
        let header_value = HeaderValue::from_str(value)
            .map_err(|e| RealtimeError::Connect(format!("invalid value for {name}: {e}")))?;
        headers.insert(header_name, header_value);
    }

    let (ws, _response) = connect_async(request)
        .await
        .map_err(|e| RealtimeError::Connect(e.to_string()))?;

    Ok(RealtimeSession { ws })
}

/// A live realtime WebSocket session.
///
/// Wraps the underlying WebSocket stream and exposes JSON send/receive.
pub struct RealtimeSession {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl RealtimeSession {
    /// Send a JSON event as a text frame.
    ///
    /// Not cancel-safe: dropping this future mid-flight (e.g. racing it in
    /// `tokio::select!`) can leave a partial frame on the connection. Let it
    /// run to completion.
    pub async fn send(&mut self, event: serde_json::Value) -> Result<(), RealtimeError> {
        let text = serde_json::to_string(&event)?;
        self.ws
            .send(Message::Text(text))
            .await
            .map_err(|e| RealtimeError::Protocol(e.to_string()))?;
        Ok(())
    }

    /// Receive the next JSON event.
    ///
    /// Returns `Ok(None)` on a clean close. Ping/Pong frames are handled
    /// automatically by the underlying library and skipped here; non-text
    /// frames other than binary JSON are skipped as well. This method is
    /// cancel-safe. Keep polling it even when idle — server pings are only
    /// answered while a read is in progress, and an unpolled session may be
    /// dropped by the server.
    pub async fn recv(&mut self) -> Result<Option<serde_json::Value>, RealtimeError> {
        while let Some(message) = self.ws.next().await {
            let message = message.map_err(|e| RealtimeError::Protocol(e.to_string()))?;
            match message {
                Message::Text(text) => {
                    let value = serde_json::from_str(text.as_str())?;
                    return Ok(Some(value));
                }
                Message::Binary(bytes) => {
                    let value = serde_json::from_slice(bytes.as_ref())?;
                    return Ok(Some(value));
                }
                Message::Close(_) => return Ok(None),
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            }
        }
        Ok(None)
    }

    /// Close the connection with a normal closure handshake.
    pub async fn close(mut self) -> Result<(), RealtimeError> {
        self.ws
            .close(None)
            .await
            .map_err(|e| RealtimeError::Protocol(e.to_string()))?;
        Ok(())
    }
}

/// Typed constructors for the most common realtime **client** events.
///
/// Each returns a [`serde_json::Value`] with the standard
/// `{"type": "...", ...}` envelope. Build any event not covered here as a
/// plain JSON value.
pub mod events {
    use serde_json::{json, Value};

    /// `{"type": "session.update", "session": <session>}`.
    pub fn session_update(session: Value) -> Value {
        json!({ "type": "session.update", "session": session })
    }

    /// `{"type": "input_audio_buffer.append", "audio": <base64_audio>}`.
    pub fn input_audio_buffer_append(base64_audio: &str) -> Value {
        json!({ "type": "input_audio_buffer.append", "audio": base64_audio })
    }

    /// `{"type": "input_audio_buffer.commit"}`.
    pub fn input_audio_buffer_commit() -> Value {
        json!({ "type": "input_audio_buffer.commit" })
    }

    /// `{"type": "conversation.item.create", "item": <item>}`.
    pub fn conversation_item_create(item: Value) -> Value {
        json!({ "type": "conversation.item.create", "item": item })
    }

    /// `{"type": "response.create"}`.
    pub fn response_create() -> Value {
        json!({ "type": "response.create" })
    }

    /// `{"type": "response.create", "response": <response>}`.
    pub fn response_create_with(response: Value) -> Value {
        json!({ "type": "response.create", "response": response })
    }
}
