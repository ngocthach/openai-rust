//! Live smoke run against a real API: each scenario prints one comparable
//! line `<name>\t<ok|ERR>\t<facts>` (shapes and types, not generated text).
//!
//! ```sh
//! OPENAI_API_KEY=sk-... OPENAI_MODEL_NAME=gpt-5.4-mini cargo run --example live-smoke
//! ```

use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use openai_compat::types::chat::ChatCompletionListParams;
use openai_compat::types::conversations::{
    ConversationCreateParams, ConversationItemsCreateParams, ConversationItemsListParams,
    ConversationUpdateParams,
};
use openai_compat::types::responses::{
    CompactResponseRequest, InputTokenCountRequest, ReasoningEffort,
};
use openai_compat::{
    ChatCompletion, ChatCompletionRequest, Client, CreateResponseRequest, InputItem, Message,
    OpenAIError, Reasoning, ResponseStreamEvent, StreamOptions,
};

type Res = Result<(), OpenAIError>;

fn ok(name: &str, facts: String) {
    println!("{name}\tok\t{facts}");
}

fn report(name: &str, result: Res) {
    if let Err(e) = result {
        let msg: String = e.to_string().chars().take(300).collect();
        println!("{name}\tERR\t{msg}");
    }
}

fn status(e: &OpenAIError) -> Option<u16> {
    match e {
        OpenAIError::Api(api) => Some(api.status),
        _ => None,
    }
}

/// Stored chat completions are eventually consistent: retry 404 briefly.
async fn retrieve_stored(client: &Client, id: &str) -> Result<ChatCompletion, OpenAIError> {
    let mut tries = 0;
    loop {
        match client.chat().completions().retrieve(id).await {
            Err(e) if status(&e) == Some(404) && tries < 9 => {
                tries += 1;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            other => return other,
        }
    }
}

fn type_of(v: &impl serde::Serialize) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v["type"].as_str().map(str::to_owned))
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    let model = std::env::var("OPENAI_MODEL_NAME")?;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let tag: HashMap<String, String> = [("smoke".to_string(), format!("rs-{secs}"))].into();
    let mut tag_updated = tag.clone();
    tag_updated.insert("updated".into(), "yes".into());

    report(
        "models",
        async {
            let m = client.models().retrieve(&model).await?;
            ok(
                "models.retrieve",
                format!("id={} object={}", m.id, m.object),
            );
            Ok(())
        }
        .await,
    );

    let mut chat_id = None;
    report(
        "chat_create",
        async {
            let req = ChatCompletionRequest::new(
                &model,
                vec![Message::user("Reply with the word: pong")],
            )
            .store(true)
            .metadata(tag.clone())
            .reasoning_effort("low");
            let c = client.chat().completions().create(req).await?;
            ok(
                "chat.create",
                format!(
                    "object={} finish={:?} has_text={} usage={}",
                    c.object,
                    c.choices.first().and_then(|c| c.finish_reason),
                    c.content().is_some_and(|t| !t.is_empty()),
                    c.usage.is_some()
                ),
            );
            chat_id = Some(c.id);
            Ok(())
        }
        .await,
    );

    report(
        "chat_stream",
        async {
            let req = ChatCompletionRequest::new(&model, vec![Message::user("Count 1 to 3")])
                .stream_options(StreamOptions {
                    include_usage: Some(true),
                });
            let mut s = client.chat().completions().create_stream(req).await?;
            let (mut text, mut usage, mut n) = (String::new(), false, 0);
            while let Some(chunk) = s.next().await {
                let chunk = chunk?;
                n += 1;
                text.push_str(chunk.content().unwrap_or_default());
                usage |= chunk.usage.is_some();
            }
            ok(
                "chat.stream",
                format!(
                    "chunks>1={} has_text={} usage={usage}",
                    n > 1,
                    !text.is_empty()
                ),
            );
            Ok(())
        }
        .await,
    );

    if let Some(cid) = chat_id.as_deref() {
        report(
            "stored_chat",
            async {
                let chats = client.chat().completions();
                let c = retrieve_stored(&client, cid).await?;
                let meta = serde_json::to_value(&c)?["metadata"].clone();
                ok(
                    "chat.retrieve",
                    format!("same_id={} metadata={meta}", c.id == cid),
                );
                let msgs = chats.messages(cid, None).await?;
                ok(
                    "chat.messages",
                    format!(
                        "count={} role={:?}",
                        msgs.data.len(),
                        msgs.data.first().map(|m| &m.message.role)
                    ),
                );
                let u = chats.update(cid, tag_updated.clone()).await?;
                ok(
                    "chat.update",
                    format!("metadata={}", serde_json::to_value(&u)?["metadata"]),
                );
                let page = chats
                    .list(Some(
                        ChatCompletionListParams::new()
                            .metadata(tag.clone())
                            .limit(5),
                    ))
                    .await?;
                ok(
                    "chat.list(metadata)",
                    format!(
                        "object={} found={}",
                        page.object.as_deref().unwrap_or("-"),
                        page.data.iter().any(|x| x.id == cid)
                    ),
                );
                let d = chats.delete(cid).await?;
                ok(
                    "chat.delete",
                    format!("deleted={} object={}", d.deleted, d.object),
                );
                Ok(())
            }
            .await,
        );
    }

    report(
        "responses_create",
        async {
            let req = CreateResponseRequest::new(&model, "Reply with the word: pong")
                .reasoning(Reasoning::effort(ReasoningEffort::Low))
                .store(true);
            let r = client.responses().create(req).await?;
            ok(
                "responses.create",
                format!(
                    "object={} status={:?} has_text={} usage={}",
                    r.object,
                    r.status,
                    !r.output_text().is_empty(),
                    r.usage.is_some()
                ),
            );
            Ok(())
        }
        .await,
    );

    report(
        "responses_stream",
        async {
            let mut s = client
                .responses()
                .create_stream(CreateResponseRequest::new(&model, "Count 1 to 3"))
                .await?;
            let (mut first, mut last, mut delta) = (None, None, false);
            while let Some(ev) = s.next().await {
                let ev = ev?;
                let name = match &ev {
                    ResponseStreamEvent::Created { .. } => "response.created",
                    ResponseStreamEvent::Completed { .. } => "response.completed",
                    ResponseStreamEvent::OutputTextDelta { .. } => {
                        delta = true;
                        "delta"
                    }
                    _ => "other",
                };
                first.get_or_insert(name);
                last = Some(name);
            }
            ok(
                "responses.stream",
                format!(
                    "first={} last={} has_delta={delta}",
                    first.unwrap_or("-"),
                    last.unwrap_or("-")
                ),
            );
            Ok(())
        }
        .await,
    );

    report(
        "count_tokens",
        async {
            let c = client
                .responses()
                .count_input_tokens(
                    InputTokenCountRequest::new()
                        .model(&model)
                        .input("Hello there, how are you?"),
                )
                .await?;
            ok(
                "responses.input_tokens",
                format!("object={} tokens>0={}", c.object, c.input_tokens > 0),
            );
            Ok(())
        }
        .await,
    );

    report(
        "compact",
        async {
            let input = vec![
                InputItem::user("My name is Ada."),
                InputItem::message("assistant", "Nice to meet you, Ada."),
                InputItem::user("What is my name?"),
            ];
            let c = client
                .responses()
                .compact(CompactResponseRequest::new(&model, input))
                .await?;
            let mut kinds: Vec<String> = c.output.iter().map(type_of).collect();
            kinds.sort();
            kinds.dedup();
            ok(
                "responses.compact",
                format!("object={} output_types={kinds:?} usage=true", c.object),
            );
            Ok(())
        }
        .await,
    );

    report(
        "conversations",
        async {
            let convs = client.conversations();
            let conv = convs
                .create(
                    ConversationCreateParams::new()
                        .metadata(tag.clone())
                        .items(vec![InputItem::user("hello")]),
                )
                .await?;
            let cid = conv.id.clone();
            ok(
                "conversations.create",
                format!("object={} metadata={:?}", conv.object, conv.metadata),
            );
            let g = convs.retrieve(&cid).await?;
            ok("conversations.retrieve", format!("same_id={}", g.id == cid));
            let u = convs
                .update(&cid, ConversationUpdateParams::new(tag_updated.clone()))
                .await?;
            ok("conversations.update", format!("metadata={:?}", u.metadata));
            let items = convs.items(&cid);
            let added = items
                .create(ConversationItemsCreateParams::new(vec![InputItem::user(
                    "second",
                )]))
                .await?;
            ok(
                "conversations.items.create",
                format!(
                    "object={} count={}",
                    added.object.as_deref().unwrap_or("-"),
                    added.data.len()
                ),
            );
            let page = items
                .list(Some(ConversationItemsListParams::new().limit(1)))
                .await?;
            let all = items
                .list_all(Some(ConversationItemsListParams::new().limit(1)))
                .await?;
            ok(
                "conversations.items.list",
                format!(
                    "page={} has_more={} all={} type={}",
                    page.data.len(),
                    page.has_more.unwrap_or_default(),
                    all.len(),
                    page.data
                        .first()
                        .map(|i| i.item_type.as_str())
                        .unwrap_or("-")
                ),
            );
            let item_id = added
                .data
                .first()
                .and_then(|i| i.id.clone())
                .unwrap_or_default();
            let it = items.retrieve(&item_id, None).await?;
            ok(
                "conversations.items.retrieve",
                format!(
                    "same_id={} type={}",
                    it.id.as_deref() == Some(&item_id),
                    it.item_type
                ),
            );
            let after = items.delete(&item_id).await?;
            ok(
                "conversations.items.delete",
                format!("object={}", after.object),
            );
            let d = convs.delete(&cid).await?;
            ok(
                "conversations.delete",
                format!("deleted={} object={}", d.deleted, d.object),
            );
            Ok(())
        }
        .await,
    );

    report(
        "error_format",
        async {
            let req = ChatCompletionRequest::new("no-such-model-xyz", vec![Message::user("x")]);
            match client.chat().completions().create(req).await {
                Err(OpenAIError::Api(e)) => ok(
                    "error.bad_model",
                    format!(
                        "status={} kind={:?} code={:?} request_id={} msg_prefix={:?}",
                        e.status,
                        e.kind,
                        e.detail.as_ref().and_then(|d| d.code.clone()),
                        e.request_id.is_some(),
                        e.to_string().chars().take(16).collect::<String>()
                    ),
                ),
                other => ok("error.bad_model", format!("UNEXPECTED {other:?}")),
            }
            Ok(())
        }
        .await,
    );

    report(
        "path_encoding",
        async {
            match client.chat().completions().retrieve("a/../../models").await {
                Err(e) => ok(
                    "path.encoded_id",
                    format!(
                        "status={:?} err={}",
                        status(&e),
                        e.to_string().chars().take(80).collect::<String>()
                    ),
                ),
                Ok(_) => ok("path.encoded_id", "status=200 (NOT encoded!)".into()),
            }
            Ok(())
        }
        .await,
    );

    Ok(())
}
