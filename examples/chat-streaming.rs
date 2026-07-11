//! Streaming chat completion: prints tokens as they arrive.
//!
//! ```sh
//! OPENAI_API_KEY=sk-... cargo run --example chat-streaming
//! ```

use std::io::Write;

use futures_util::StreamExt;
use openai_compat::{ChatCompletionRequest, Client, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;

    let request = ChatCompletionRequest::new(
        "gpt-4o-mini",
        vec![Message::user(
            "Count from 1 to 10 slowly, one number per line.",
        )],
    );

    let mut stream = client.chat().completions().create_stream(request).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = chunk.content() {
            print!("{content}");
            std::io::stdout().flush()?;
        }
    }
    println!();
    Ok(())
}
