//! Responses API: a basic create, then a streaming create that inspects the
//! terminal event (`Completed`/`Failed`/`Incomplete` are all typed `Ok`
//! variants, not stream errors).
//!
//! ```sh
//! OPENAI_API_KEY=sk-... cargo run --example responses
//! ```
//! Works with any OpenAI-compatible provider via OPENAI_BASE_URL.

use futures_util::StreamExt;
use openai_compat::types::responses::ResponseStreamEvent;
use openai_compat::{Client, CreateResponseRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;

    // Non-streaming create.
    let request = CreateResponseRequest::new(
        "gpt-4o-mini",
        "In one sentence, what is Rust's borrow checker?",
    )
    .temperature(0.7);

    let response = client.responses().create(request).await?;
    println!("{}", response.output_text());
    if let Some(usage) = &response.usage {
        println!(
            "-- {} input + {} output = {} tokens",
            usage.input_tokens, usage.output_tokens, usage.total_tokens
        );
    }

    // Streaming create: print text deltas as they arrive, then inspect the
    // terminal event. A failed response is a typed `Ok(Failed{..})`, not a
    // stream `Err` — callers must check the variant themselves.
    println!("\n--- streaming ---");
    let request =
        CreateResponseRequest::new("gpt-4o-mini", "Count from 1 to 5, one number per line.");
    let mut stream = client.responses().create_stream(request).await?;
    while let Some(event) = stream.next().await {
        match event? {
            ResponseStreamEvent::OutputTextDelta { delta, .. } => print!("{delta}"),
            ResponseStreamEvent::Completed { .. } => println!("\n[completed]"),
            ResponseStreamEvent::Failed { response, .. } => {
                println!("\n[failed] {:?}", response.error);
            }
            ResponseStreamEvent::Incomplete { response, .. } => {
                println!("\n[incomplete] {:?}", response.incomplete_details);
            }
            _ => {}
        }
    }

    Ok(())
}
