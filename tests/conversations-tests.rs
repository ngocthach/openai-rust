//! Integration tests for the conversations resource against a wiremock
//! server: CRUD, items (create/retrieve/list/delete), `include[]` encoding,
//! and `last_id`-driven pagination.

use std::collections::HashMap;

use openai_compat::types::conversations::{
    ConversationCreateParams, ConversationItemsCreateParams, ConversationItemsListParams,
    ConversationUpdateParams,
};
use openai_compat::types::responses::{InputItem, ResponseIncludable};
use openai_compat::{Client, HasId, OpenAIError};
use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-test-key")
        .base_url(format!("{}/v1", server.uri()))
        .max_retries(1)
        .build()
        .unwrap()
}

fn conversation_body() -> serde_json::Value {
    json!({
        "id": "conv_1", "object": "conversation", "created_at": 1700000000,
        "metadata": {"topic": "demo"}
    })
}

fn item(id: &str) -> serde_json::Value {
    json!({"id": id, "type": "message", "role": "user",
           "content": [{"type": "input_text", "text": "hi"}]})
}

#[tokio::test]
async fn conversation_create_posts_items_and_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/conversations"))
        .and(body_json(json!({
            "items": [{"type": "message", "role": "user", "content": "hi"}],
            "metadata": {"topic": "demo"}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(conversation_body()))
        .expect(1)
        .mount(&server)
        .await;

    let params = ConversationCreateParams::new()
        .items(vec![InputItem::user("hi")])
        .metadata(HashMap::from([("topic".to_string(), "demo".to_string())]));
    let conv = client_for(&server)
        .conversations()
        .create(params)
        .await
        .unwrap();
    assert_eq!(conv.id, "conv_1");
    assert_eq!(conv.object, "conversation");
}

#[tokio::test]
async fn conversation_retrieve_update_delete() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/conversations/conv_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(conversation_body()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/conversations/conv_1"))
        .and(body_json(json!({"metadata": {"k": "v"}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(conversation_body()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1/conversations/conv_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "conv_1", "object": "conversation.deleted", "deleted": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let conversations = client.conversations();
    assert_eq!(conversations.retrieve("conv_1").await.unwrap().id, "conv_1");
    let update = ConversationUpdateParams::new(HashMap::from([("k".into(), "v".into())]));
    conversations.update("conv_1", update).await.unwrap();
    let deleted = conversations.delete("conv_1").await.unwrap();
    assert!(deleted.deleted);
}

#[tokio::test]
async fn conversation_rejects_unsafe_ids_without_a_request() {
    let server = MockServer::start().await;
    let client = client_for(&server);
    for bad in ["", "..", "."] {
        let err = client.conversations().retrieve(bad).await.unwrap_err();
        assert!(matches!(err, OpenAIError::Config(_)), "{bad:?}: {err}");
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn items_create_sends_body_and_include_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/conversations/conv_1/items"))
        .and(query_param("include[]", "message.output_text.logprobs"))
        .and(body_json(json!({
            "items": [{"type": "message", "role": "user", "content": "hi"}]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": [item("msg_1")],
            "first_id": "msg_1", "last_id": "msg_1", "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let params = ConversationItemsCreateParams::new(vec![InputItem::user("hi")])
        .include(vec![ResponseIncludable::MessageOutputTextLogprobs]);
    let list = client_for(&server)
        .conversations()
        .items("conv_1")
        .create(params)
        .await
        .unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id(), Some("msg_1"));
    assert_eq!(list.data[0].item_type, "message");
    assert_eq!(list.data[0].rest["role"], "user");
}

#[tokio::test]
async fn item_retrieve_and_delete() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/conversations/conv_1/items/msg_1"))
        .and(query_param("include[]", "reasoning.encrypted_content"))
        .respond_with(ResponseTemplate::new(200).set_body_json(item("msg_1")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1/conversations/conv_1/items/msg_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(conversation_body()))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    let items = client.conversations().items("conv_1");
    let found = items
        .retrieve(
            "msg_1",
            Some(vec![ResponseIncludable::ReasoningEncryptedContent]),
        )
        .await
        .unwrap();
    assert_eq!(found.id.as_deref(), Some("msg_1"));
    assert_eq!(items.delete("msg_1").await.unwrap().id, "conv_1");
}

#[tokio::test]
async fn items_list_sends_cursor_and_include_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/conversations/conv_1/items"))
        .and(query_param("after", "msg_0"))
        .and(query_param("limit", "5"))
        .and(query_param("order", "asc"))
        .and(query_param("include[]", "web_search_call.action.sources"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": [item("msg_1")],
            "first_id": "msg_1", "last_id": "msg_1", "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let params = ConversationItemsListParams::new()
        .after("msg_0")
        .limit(5)
        .order("asc")
        .include(vec![ResponseIncludable::WebSearchCallActionSources]);
    let page = client_for(&server)
        .conversations()
        .items("conv_1")
        .list(Some(params))
        .await
        .unwrap();
    assert_eq!(page.data.len(), 1);
}

#[tokio::test]
async fn items_list_all_follows_envelope_last_id() {
    let server = MockServer::start().await;
    // The envelope cursor ("cursor_a") deliberately differs from the last
    // item's id ("msg_2"): the next request must carry the envelope value.
    Mock::given(method("GET"))
        .and(path("/v1/conversations/conv_1/items"))
        .and(query_param("after", "cursor_a"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": [item("msg_3")],
            "first_id": "msg_3", "last_id": "msg_3", "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/conversations/conv_1/items"))
        .and(query_param_is_missing("after"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": [item("msg_1"), item("msg_2")],
            "first_id": "msg_1", "last_id": "cursor_a", "has_more": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let all = client_for(&server)
        .conversations()
        .items("conv_1")
        .list_all(None)
        .await
        .unwrap();
    let ids: Vec<_> = all.iter().filter_map(|i| i.id()).collect();
    assert_eq!(ids, ["msg_1", "msg_2", "msg_3"]);
}

#[tokio::test]
async fn items_list_all_errors_when_more_pages_but_no_last_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/conversations/conv_1/items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": [item("msg_1")], "has_more": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client_for(&server)
        .conversations()
        .items("conv_1")
        .list_all(None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("last_id"), "got: {err}");
}

#[tokio::test]
async fn items_list_all_does_not_refetch_on_empty_last_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/conversations/conv_1/items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": [item("msg_1")], "last_id": "", "has_more": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client_for(&server)
        .conversations()
        .items("conv_1")
        .list_all(None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("last_id"), "got: {err}");
}
