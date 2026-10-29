# openai-compat

[![crates.io](https://img.shields.io/crates/v/openai-compat.svg)](https://crates.io/crates/openai-compat)
[![docs.rs](https://docs.rs/openai-compat/badge.svg)](https://docs.rs/openai-compat)
[![license](https://img.shields.io/crates/l/openai-compat.svg)](./LICENSE)

Async Rust client for the OpenAI API and any OpenAI-compatible LLM provider,
modeled on the official [openai-python](https://github.com/openai/openai-python) SDK.

Full API documentation: [docs.rs/openai-compat](https://docs.rs/openai-compat)

## Features

- **Chat completions** — full request surface (tools, `response_format` /
  JSON schema, penalties, logprobs, seed, stop, ...) with typed responses
- **Responses API** — `create`/`retrieve`/`delete`/`cancel`, background mode
  with resumable streaming, stateful chaining via `previous_response_id`, and
  `input_items` pagination
- **Streaming** — server-sent events exposed as a `futures::Stream` of typed
  chunks, terminating on `[DONE]` and surfacing mid-stream errors
- **Embeddings, models, moderations, legacy completions, images, files
  (multipart upload/download), audio (TTS + transcription)**
- **Batches, resumable uploads, fine-tuning jobs, vector stores, assistants
  (beta v2: threads, messages, runs, run steps)**
- **Multimodal messages** — text, image, and audio content parts
- **Webhooks** — HMAC-SHA256 signature verification (constant-time, timestamp
  tolerance) matching the Python SDK
- **Azure OpenAI** — `api-version` query, `api-key`/Entra ID auth, and
  deployment-based paths via the same client builder
- **Realtime API** — WebSocket sessions (tokio-tungstenite/rustls) with JSON
  events and typed event constructors
- **Automatic retries** — mirrors the Python SDK: 408/409/429/5xx and
  connection errors, exponential backoff with jitter (0.5s → 8s),
  `Retry-After`/`retry-after-ms`/`x-should-retry` support, 2 retries by default
- **Typed errors** — status-specific error kinds with parsed
  `{message, type, param, code}` detail and `x-request-id`
- **Any provider** — set `base_url` to use any OpenAI-compatible endpoint
- **Escape hatch** — generic `get`/`post`/`delete` for endpoints not yet typed

## Installation

```toml
[dependencies]
openai-compat = "0.5"
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"   # only needed for streaming
```

## Quick start

```rust,no_run
use openai_compat::{ChatCompletionRequest, Client, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads OPENAI_API_KEY (and optional OPENAI_BASE_URL, OPENAI_ORG_ID,
    // OPENAI_PROJECT_ID) from the environment.
    let client = Client::new()?;

    let request = ChatCompletionRequest::new(
        "gpt-4o-mini",
        vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello!"),
        ],
    )
    .temperature(0.7);

    let completion = client.chat().completions().create(request).await?;
    println!("{}", completion.content().unwrap_or_default());
    Ok(())
}
```

### Explicit configuration / other providers

```rust,no_run
use openai_compat::Client;
use std::time::Duration;

# fn main() -> Result<(), openai_compat::OpenAIError> {
let client = Client::builder()
    .api_key("sk-...")
    .base_url("https://openrouter.ai/api/v1") // any OpenAI-compatible endpoint
    .timeout(Duration::from_secs(120))
    .max_retries(3)
    .header("X-Custom", "value")
    .build()?;
# Ok(())
# }
```

### Streaming

```rust,no_run
use futures_util::StreamExt;
use openai_compat::{ChatCompletionRequest, Client, Message};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
# let client = Client::new()?;
let request = ChatCompletionRequest::new("gpt-4o-mini", vec![Message::user("Hi")]);
let mut stream = client.chat().completions().create_stream(request).await?;

while let Some(chunk) = stream.next().await {
    if let Some(content) = chunk?.content() {
        print!("{content}");
    }
}
# Ok(())
# }
```

### Tool calling

```rust,no_run
use openai_compat::{ChatCompletionRequest, Client, Message, Tool, ToolChoice};
use serde_json::json;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
# let client = Client::new()?;
let request = ChatCompletionRequest::new("gpt-4o-mini", vec![Message::user("Weather in Hanoi?")])
    .tools(vec![Tool::function(
        "get_weather",
        "Get current weather for a city",
        json!({"type": "object", "properties": {"city": {"type": "string"}}}),
    )])
    .tool_choice(ToolChoice::Auto);

let completion = client.chat().completions().create(request).await?;
if let Some(calls) = &completion.choices[0].message.tool_calls {
    for call in calls {
        // `function` is `None` for custom tool calls (see `call.custom`).
        if let Some(function) = &call.function {
            println!("{} -> {}", function.name, function.arguments);
        }
    }
}
# Ok(())
# }
```

### Responses API

```rust,no_run
use openai_compat::{Client, CreateResponseRequest};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
# let client = Client::new()?;
let request = CreateResponseRequest::new("gpt-4o-mini", "Hello!");
let response = client.responses().create(request).await?;
println!("{}", response.output_text());

// Stateful multi-turn chaining: continue from a prior response instead of
// resending the full conversation history.
let follow_up = CreateResponseRequest::new("gpt-4o-mini", "And in French?")
    .previous_response_id(response.id);
client.responses().create(follow_up).await?;
# Ok(())
# }
```

Streaming uses the same `EventStream` as chat, but yields a tagged union of
~12 typed events (`Created`, `OutputTextDelta`, `Completed`, `Failed`, ...)
instead of homogeneous delta chunks; unrecognized event types deserialize to
`ResponseStreamEvent::Unknown` rather than erroring. `Failed`/`Incomplete` are
typed `Ok` variants, not stream errors — inspect the variant to detect them.
See `examples/responses.rs` for a full streaming example, and
`client.responses().input_items(id).list_all(None)` for paginating the items
that produced a response.

### Other resources

```rust,no_run
use openai_compat::types::embeddings::EmbeddingRequest;
use openai_compat::types::files::FileUpload;
use openai_compat::types::audio::SpeechRequest;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
# let client = openai_compat::Client::new()?;
// Embeddings
let embeddings = client
    .embeddings()
    .create(EmbeddingRequest::new("text-embedding-3-small", "hello world"))
    .await?;

// Models
let models = client.models().list().await?;

// Files (multipart upload)
let file = client
    .files()
    .create(FileUpload::from_path("data.jsonl").await?, "fine-tune")
    .await?;

// Text-to-speech (binary response)
let audio = client
    .audio()
    .speech(SpeechRequest::new("tts-1", "Hello!", "alloy"))
    .await?;
# Ok(())
# }
```

### Error handling

```rust,no_run
use openai_compat::{ApiErrorKind, OpenAIError};

# async fn run(client: openai_compat::Client, req: openai_compat::ChatCompletionRequest) {
match client.chat().completions().create(req).await {
    Ok(completion) => println!("{:?}", completion.content()),
    Err(OpenAIError::Api(err)) => {
        // 4xx/5xx with parsed body: err.status, err.kind, err.detail, err.request_id
        if err.kind == ApiErrorKind::RateLimit {
            eprintln!("rate limited: {err}");
        }
    }
    Err(OpenAIError::Timeout) => eprintln!("request timed out"),
    Err(other) => eprintln!("{other}"),
}
# }
```

### Multimodal messages

```rust,no_run
use openai_compat::{ChatCompletionRequest, ContentPart, Message};

# fn build() -> ChatCompletionRequest {
ChatCompletionRequest::new(
    "gpt-4o",
    vec![Message::user(vec![
        ContentPart::text("What is in this image?"),
        ContentPart::image_url("https://example.com/photo.png"),
    ])],
)
# }
```

### Batches, fine-tuning, vector stores, assistants

```rust,no_run
use openai_compat::types::batches::BatchCreateParams;
use openai_compat::types::fine_tuning::FineTuningJobRequest;

# async fn run(client: openai_compat::Client) -> Result<(), openai_compat::OpenAIError> {
let batch = client
    .batches()
    .create(BatchCreateParams::new("file-abc", "/v1/chat/completions", "24h"))
    .await?;

let job = client
    .fine_tuning()
    .jobs()
    .create(FineTuningJobRequest::new("gpt-4o-mini-2024-07-18", "file-train"))
    .await?;

let stores = client.vector_stores().list(None).await?;
let assistants = client.assistants().list(None).await?; // OpenAI-Beta: assistants=v2 sent automatically
# Ok(())
# }
```

### Webhooks

```rust,no_run
use openai_compat::webhooks::{Webhooks, WebhookHeaders};

# fn verify(payload: &[u8], headers: &WebhookHeaders) -> bool {
let webhooks = Webhooks::new(&std::env::var("OPENAI_WEBHOOK_SECRET").unwrap()).unwrap();
webhooks.unwrap(payload, headers).is_ok() // verifies signature, then parses the event
# }
```

### Azure OpenAI

```rust,no_run
# fn main() -> Result<(), openai_compat::OpenAIError> {
let client = openai_compat::Client::builder()
    .azure("https://my-resource.openai.azure.com", "2024-06-01")
    .azure_deployment("my-gpt4o")   // optional: else derived from the request's `model`
    .build()?;                      // key from AZURE_OPENAI_API_KEY, or .azure_ad_token(...)
# Ok(())
# }
```

### Realtime

```rust,no_run
use openai_compat::realtime::events;

# async fn run(client: openai_compat::Client) -> Result<(), Box<dyn std::error::Error>> {
let mut session = client.connect_realtime("gpt-4o-realtime-preview").await?;
session.send(events::response_create()).await?;
while let Some(event) = session.recv().await? {
    println!("{}", event["type"]);
}
# Ok(())
# }
```

## Examples

```sh
OPENAI_API_KEY=sk-... cargo run --example chat
OPENAI_API_KEY=sk-... cargo run --example chat-streaming
OPENAI_API_KEY=sk-... cargo run --example tool-calling
OPENAI_API_KEY=sk-... cargo run --example responses
```

## Scope

v0.5 tracks `openai-python` v3.40.0 (see `docs/upstream-sync.md`). It ports the full core client surface: chat (incl.
multimodal content parts), the Responses API (incl. streaming, background
mode, `input_items` pagination), embeddings, models, moderations, legacy
completions, images (generate), files, audio (speech/transcriptions), batches,
resumable uploads, fine-tuning jobs, vector stores, assistants (beta v2),
conversations (+ items), stored chat completions (retrieve/update/list/delete/
messages), Responses `compact` and input token counting,
webhook signature verification, Azure OpenAI mode, realtime WebSockets,
retries, streaming, and cursor pagination.

Deliberately simplified: assistants streaming runs and the fully-typed
realtime event surface are not modeled (events are `serde_json::Value` with
typed constructors); deep polymorphic fields (graders, chunking filters,
step details) are `serde_json::Value`. Responses API v1 covers
create/retrieve/delete/cancel/streaming/`compact`/`count_input_tokens`/
`input_items`, with the built-in tools beyond
`web_search`/`file_search`/`code_interpreter`, the `parse()` structured-output
wrapper, and the Responses-over-WebSocket connection left as future work. For
untyped endpoints use the `client.get::<serde_json::Value>(...)` /
`client.post(...)` escape hatch.

## License

Apache-2.0
