//! The eight list positions schema 2.0 declares with non-null elements.
//!
//! Each test mounts a wiremock endpoint returning a `null` element in exactly
//! one of those positions, with **no** `errors` key. That response breaks the
//! schema, so the SDK must fail it with `Err(Decode)` instead of passing the
//! null through or eliding it. The two lists that are themselves nullable
//! (`EventOutput.eventData`, `ActionOutput.actionData`) still decode a whole-list
//! `null` to `None`.

use std::time::Duration;

use mina_archive_sdk::{
    ActionFilterOptionsInput, ArchiveClient, ClientConfig, Error, EventFilterOptionsInput,
    GetBlocksOptions,
};
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Start a server that answers every POST with `body`.
async fn server_returning(body: Value) -> (MockServer, ArchiveClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let client = ArchiveClient::with_config(
        ClientConfig::new(server.uri())
            .attempts(1)
            .retry_delay(Duration::from_millis(1))
            .timeout(Duration::from_secs(5)),
    );
    (server, client)
}

fn assert_decode_error<T: std::fmt::Debug>(result: mina_archive_sdk::Result<T>) {
    match result {
        Err(Error::Decode { .. }) => {}
        other => panic!("a null element must be a decode error, got {other:?}"),
    }
}

fn transaction_info(ids: Value) -> Value {
    json!({
        "status": "applied",
        "hash": "h",
        "memo": "m",
        "authorizationKind": "Proof",
        "sequenceNumber": 0,
        "zkappAccountUpdateIds": ids
    })
}

fn event_data(data: Value, info: Value) -> Value {
    json!({ "accountUpdateId": "1", "transactionInfo": info, "data": data })
}

fn events_body(event_data: Value) -> Value {
    json!({ "data": { "events": [{ "blockInfo": null, "eventData": event_data }] } })
}

fn actions_body(action_data: Value) -> Value {
    json!({
        "data": {
            "actions": [{
                "blockInfo": null,
                "transactionInfo": null,
                "actionData": action_data,
                "actionState": {
                    "actionStateOne": null,
                    "actionStateTwo": null,
                    "actionStateThree": null,
                    "actionStateFour": null,
                    "actionStateFive": null
                }
            }]
        }
    })
}

async fn get_events(body: Value) -> mina_archive_sdk::Result<Vec<mina_archive_sdk::EventOutput>> {
    let (_server, client) = server_returning(body).await;
    client
        .get_events(EventFilterOptionsInput::for_address("B62q..."))
        .await
}

async fn get_actions(body: Value) -> mina_archive_sdk::Result<Vec<mina_archive_sdk::ActionOutput>> {
    let (_server, client) = server_returning(body).await;
    client
        .get_actions(ActionFilterOptionsInput::for_address("B62q..."))
        .await
}

/// `Query.events: [EventOutput!]!`
#[tokio::test]
async fn null_event_element_is_a_decode_error() {
    assert_decode_error(get_events(json!({ "data": { "events": [null] } })).await);
}

/// `Query.actions: [ActionOutput!]!`
#[tokio::test]
async fn null_action_element_is_a_decode_error() {
    assert_decode_error(get_actions(json!({ "data": { "actions": [null] } })).await);
}

/// `Query.blocks: [Block!]!`
#[tokio::test]
async fn null_block_element_is_a_decode_error() {
    let (_server, client) = server_returning(json!({ "data": { "blocks": [null] } })).await;
    assert_decode_error(client.get_blocks(GetBlocksOptions::default()).await);
}

/// `EventOutput.eventData: [EventData!]`
#[tokio::test]
async fn null_event_data_element_is_a_decode_error() {
    assert_decode_error(get_events(events_body(json!([null]))).await);
}

/// `ActionOutput.actionData: [ActionData!]`
#[tokio::test]
async fn null_action_data_element_is_a_decode_error() {
    assert_decode_error(get_actions(actions_body(json!([null]))).await);
}

/// `EventData.data: [String!]!`
#[tokio::test]
async fn null_event_data_member_is_a_decode_error() {
    let body = events_body(json!([event_data(json!(["0x1", null]), Value::Null)]));
    assert_decode_error(get_events(body).await);
}

/// `ActionData.data: [String!]!`
#[tokio::test]
async fn null_action_data_member_is_a_decode_error() {
    let body = actions_body(json!([event_data(json!([null]), Value::Null)]));
    assert_decode_error(get_actions(body).await);
}

/// `TransactionInfo.zkappAccountUpdateIds: [Int!]!`
#[tokio::test]
async fn null_zkapp_account_update_id_is_a_decode_error() {
    let info = transaction_info(json!([42, null]));
    let body = events_body(json!([event_data(json!(["0x1"]), info)]));
    assert_decode_error(get_events(body).await);
}

/// The outer `eventData` and `actionData` lists stay nullable in schema 2.0.
#[tokio::test]
async fn null_event_data_and_action_data_lists_decode_to_none() {
    let events = get_events(events_body(Value::Null))
        .await
        .expect("a null eventData list is legal");
    assert!(events[0].event_data.is_none());

    let actions = get_actions(actions_body(Value::Null))
        .await
        .expect("a null actionData list is legal");
    assert!(actions[0].action_data.is_none());
}

/// A fully populated response decodes, and an empty string member stays `""`.
#[tokio::test]
async fn non_null_elements_decode() {
    let info = transaction_info(json!([42, 7]));
    let body = events_body(json!([event_data(json!(["0x1", ""]), info)]));
    let events = get_events(body).await.expect("a legal response decodes");

    let data = &events[0].event_data.as_ref().expect("eventData present")[0];
    assert_eq!(data.data, vec!["0x1".to_string(), String::new()]);
    let info = data
        .transaction_info
        .as_ref()
        .expect("transactionInfo present");
    assert_eq!(info.zkapp_account_update_ids, vec![42, 7]);
}
