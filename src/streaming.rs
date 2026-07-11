//! Server-sent events decoding and the async [`EventStream`] adapter,
//! mirroring `_streaming.py` (`SSEDecoder` and `Stream.__stream__`).

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;

use crate::error::OpenAIError;

/// A single decoded server-sent event.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerSentEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

/// Incremental SSE parser. Push bytes in with [`SseDecoder::feed`]; complete
/// events are returned as they are terminated by a blank line.
///
/// Mirrors `_streaming.py::SSEDecoder` field handling: `event`, `data`
/// (multi-line joined with `\n`), `id` (ignored if it contains NUL), `retry`,
/// comment lines (leading `:`), and a single leading space stripped from
/// values.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buf: Vec<u8>,
    event: Option<String>,
    data: Vec<String>,
    id: Option<String>,
    retry: Option<u64>,
    bom_checked: bool,
}

/// Upper bound on bytes buffered while waiting for a line terminator —
/// guards against a broken or malicious server streaming an endless line.
const MAX_SSE_BUFFER: usize = 16 * 1024 * 1024;

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bytes currently buffered awaiting a line terminator.
    pub fn buffered_len(&self) -> usize {
        self.buf.len()
    }

    /// Feed a chunk of bytes; returns all events completed by this chunk.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<ServerSentEvent> {
        self.buf.extend_from_slice(chunk);

        // Per the SSE spec, strip one leading U+FEFF byte-order mark.
        if !self.bom_checked && self.buf.len() >= 3 {
            if self.buf.starts_with(&[0xEF, 0xBB, 0xBF]) {
                self.buf.drain(..3);
            }
            self.bom_checked = true;
        }

        let mut events = Vec::new();

        // Consume complete lines; terminators are \r\n, \n, or lone \r.
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n' || b == b'\r') {
            let term_len = if self.buf[pos] == b'\r' {
                if pos + 1 == self.buf.len() {
                    // Might be the first half of a \r\n split across chunks.
                    break;
                }
                if self.buf[pos + 1] == b'\n' {
                    2
                } else {
                    1
                }
            } else {
                1
            };

            let line = String::from_utf8_lossy(&self.buf[..pos]).into_owned();
            self.buf.drain(..pos + term_len);

            if let Some(event) = self.process_line(&line) {
                events.push(event);
            }
        }
        events
    }

    /// Flush any trailing buffered line at end of stream.
    pub fn flush(&mut self) -> Vec<ServerSentEvent> {
        let mut events = Vec::new();
        // A held-back lone \r (possible split \r\n) is a terminator, not data.
        if self.buf.last() == Some(&b'\r') {
            self.buf.pop();
        }
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&self.buf).into_owned();
            self.buf.clear();
            if let Some(event) = self.process_line(&line) {
                events.push(event);
            }
        }
        // Dispatch a final unterminated event if any data was accumulated.
        if !self.data.is_empty() || self.event.is_some() {
            events.push(self.dispatch());
        }
        events
    }

    fn process_line(&mut self, line: &str) -> Option<ServerSentEvent> {
        if line.is_empty() {
            // Blank line: dispatch the accumulated event.
            if self.event.is_none() && self.data.is_empty() && self.retry.is_none() {
                return None;
            }
            return Some(self.dispatch());
        }
        if line.starts_with(':') {
            return None; // comment
        }

        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (line, ""),
        };

        match field {
            "event" => self.event = Some(value.to_string()),
            "data" => self.data.push(value.to_string()),
            "id" => {
                if !value.contains('\0') {
                    self.id = Some(value.to_string());
                }
            }
            "retry" => {
                if let Ok(retry) = value.parse() {
                    self.retry = Some(retry);
                }
            }
            _ => {} // unknown field: ignored per spec
        }
        None
    }

    fn dispatch(&mut self) -> ServerSentEvent {
        ServerSentEvent {
            event: self.event.take(),
            data: std::mem::take(&mut self.data).join("\n"),
            id: self.id.clone(),
            retry: self.retry.take(),
        }
    }
}

/// An async stream of deserialized SSE items from a streaming API response.
///
/// Terminates on the `data: [DONE]` sentinel. A mid-stream `{"error": ...}`
/// payload is surfaced as [`OpenAIError::Api`]-style [`OpenAIError::Stream`]
/// and ends the stream, mirroring `_streaming.py::Stream.__stream__`.
pub struct EventStream<T> {
    bytes: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    decoder: SseDecoder,
    pending: VecDeque<ServerSentEvent>,
    done: bool,
    bytes_exhausted: bool,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T: DeserializeOwned> EventStream<T> {
    pub(crate) fn new(response: reqwest::Response) -> Self {
        Self {
            bytes: Box::pin(response.bytes_stream()),
            decoder: SseDecoder::new(),
            pending: VecDeque::new(),
            done: false,
            bytes_exhausted: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Convert one SSE event into a stream item.
    /// `None` means the `[DONE]` sentinel was seen.
    fn item_from_event(event: ServerSentEvent) -> Option<Result<T, OpenAIError>> {
        if event.data.starts_with("[DONE]") {
            return None;
        }
        let value: serde_json::Value = match serde_json::from_str(&event.data) {
            Ok(value) => value,
            Err(err) => {
                return Some(Err(OpenAIError::Stream(format!(
                    "invalid JSON in stream: {err}"
                ))))
            }
        };
        if let Some(error) = value.get("error").filter(|e| !e.is_null()) {
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .filter(|m| !m.is_empty())
                .unwrap_or("An error occurred during streaming");
            return Some(Err(OpenAIError::Stream(message.to_string())));
        }
        match serde_json::from_value(value) {
            Ok(item) => Some(Ok(item)),
            Err(err) => Some(Err(OpenAIError::Json(err))),
        }
    }
}

impl<T: DeserializeOwned> Stream for EventStream<T> {
    type Item = Result<T, OpenAIError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if this.done {
                return Poll::Ready(None);
            }

            if let Some(event) = this.pending.pop_front() {
                // Skip events with no data (e.g. keep-alive comments produce none).
                if event.data.is_empty() {
                    continue;
                }
                match Self::item_from_event(event) {
                    Some(Ok(item)) => return Poll::Ready(Some(Ok(item))),
                    Some(Err(err)) => {
                        this.done = true;
                        return Poll::Ready(Some(Err(err)));
                    }
                    None => {
                        this.done = true;
                        return Poll::Ready(None);
                    }
                }
            }

            // Never poll the byte stream again after it has completed.
            if this.bytes_exhausted {
                this.done = true;
                return Poll::Ready(None);
            }

            match this.bytes.as_mut().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(chunk))) => {
                    this.pending.extend(this.decoder.feed(&chunk));
                    if this.decoder.buffered_len() > MAX_SSE_BUFFER {
                        this.done = true;
                        return Poll::Ready(Some(Err(OpenAIError::Stream(format!(
                            "SSE line exceeded the {MAX_SSE_BUFFER} byte buffer limit"
                        )))));
                    }
                }
                Poll::Ready(Some(Err(err))) => {
                    this.done = true;
                    return Poll::Ready(Some(Err(if err.is_timeout() {
                        OpenAIError::Timeout
                    } else {
                        OpenAIError::Connection(err.to_string())
                    })));
                }
                Poll::Ready(None) => {
                    this.bytes_exhausted = true;
                    this.pending.extend(this.decoder.flush());
                }
            }
        }
    }
}

impl<T: DeserializeOwned + Send + 'static> EventStream<T> {
    /// Collect all remaining items into a `Vec`, failing on the first error.
    pub async fn collect_all(mut self) -> Result<Vec<T>, OpenAIError> {
        let mut items = Vec::new();
        while let Some(item) = self.next().await {
            items.push(item?);
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_simple_event() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data: {\"x\":1}\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "{\"x\":1}");
        assert_eq!(events[0].event, None);
    }

    #[test]
    fn decodes_event_split_across_chunks() {
        let mut decoder = SseDecoder::new();
        assert!(decoder.feed(b"data: {\"x\"").is_empty());
        assert!(decoder.feed(b":1}\n").is_empty());
        let events = decoder.feed(b"\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "{\"x\":1}");
    }

    #[test]
    fn handles_crlf_and_cr_terminators() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data: a\r\n\r\ndata: b\r\rdata: c\n\n");
        assert_eq!(
            events.iter().map(|e| e.data.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn crlf_split_across_chunks() {
        let mut decoder = SseDecoder::new();
        assert!(decoder.feed(b"data: a\r").is_empty());
        let events = decoder.feed(b"\n\r\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "a");
    }

    #[test]
    fn multiline_data_joined_with_newline() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data: line1\ndata: line2\n\n");
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn parses_event_id_retry_and_comments() {
        let mut decoder = SseDecoder::new();
        let events =
            decoder.feed(b": keep-alive\nevent: message\nid: 42\nretry: 100\ndata: hi\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("message"));
        assert_eq!(events[0].id.as_deref(), Some("42"));
        assert_eq!(events[0].retry, Some(100));
        assert_eq!(events[0].data, "hi");
    }

    #[test]
    fn strips_single_leading_space_only() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data:  padded\ndata:none\n\n");
        assert_eq!(events[0].data, " padded\nnone");
    }

    #[test]
    fn strips_leading_bom() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"\xEF\xBB\xBFdata: first\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "first");
        // Only the first BOM is stripped; later ones are data.
        let events = decoder.feed(b"data: \xEF\xBB\xBFx\n\n");
        assert_eq!(events[0].data, "\u{FEFF}x");
    }

    #[test]
    fn flush_emits_trailing_event() {
        let mut decoder = SseDecoder::new();
        assert!(decoder.feed(b"data: tail").is_empty());
        let events = decoder.flush();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "tail");
    }
}
