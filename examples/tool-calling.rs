//! Tool calling: the model requests a function call, we execute it and send
//! the result back for a final answer.
//!
//! ```sh
//! OPENAI_API_KEY=sk-... cargo run --example tool-calling
//! ```

use openai_compat::{ChatCompletionRequest, Client, Message, Tool, ToolChoice};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;

    let tools = vec![Tool::function(
        "get_weather",
        "Get the current weather for a city",
        json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"]
        }),
    )];

    let mut messages = vec![Message::user("What's the weather in Hanoi right now?")];
    let request = ChatCompletionRequest::new("gpt-4o-mini", messages.clone())
        .tools(tools.clone())
        .tool_choice(ToolChoice::Auto);

    let completion = client.chat().completions().create(request).await?;
    let message = &completion.choices[0].message;

    let Some(tool_calls) = &message.tool_calls else {
        println!("{}", message.content.as_deref().unwrap_or("<no content>"));
        return Ok(());
    };

    // Replay the assistant turn, then answer each tool call.
    messages.push(Message::assistant_tool_calls(tool_calls.clone()));
    for call in tool_calls {
        println!("model called {}({})", call.function.name, call.function.arguments);
        // A real app would dispatch on call.function.name here.
        messages.push(Message::tool(
            json!({"temperature_c": 31, "condition": "sunny"}).to_string(),
            call.id.clone(),
        ));
    }

    let request = ChatCompletionRequest::new("gpt-4o-mini", messages).tools(tools);
    let completion = client.chat().completions().create(request).await?;
    println!("{}", completion.content().unwrap_or("<no content>"));
    Ok(())
}
