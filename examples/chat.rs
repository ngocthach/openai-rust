//! Basic chat completion.
//!
//! ```sh
//! OPENAI_API_KEY=sk-... cargo run --example chat
//! ```
//! Works with any OpenAI-compatible provider via OPENAI_BASE_URL.

use openai_rust::{ChatCompletionRequest, Client, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;

    let request = ChatCompletionRequest::new(
        "gpt-4o-mini",
        vec![
            Message::system("You are a concise assistant."),
            Message::user("In one sentence, what is Rust's borrow checker?"),
        ],
    )
    .temperature(0.7)
    .max_completion_tokens(100);

    let completion = client.chat().completions().create(request).await?;

    println!("{}", completion.content().unwrap_or("<no content>"));
    if let Some(usage) = completion.usage {
        println!(
            "-- {} prompt + {} completion = {} tokens",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }
    Ok(())
}
