//! The six SDL-nullable positions that used to be modelled as non-optional.
//!
//! Each test mounts a wiremock endpoint returning a `null` in exactly one of
//! those positions, with **no** `errors` key — a fully legal response under
//! `schema.graphql`. Before the fix every one of these failed the whole query
//! with `Err(Decode)`; now each returns `Ok` and exposes the null as `None`
//! rather than eliding it.

use std::time::Duration;

use mina_archive_sdk::{
    ActionFilterOptionsInput, ArchiveClient, ClientConfig, EventFilterOptionsInput,
    GetBlocksOptions,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Start a server that answers every POST with `body`.
async fn server_returning(body: serde_json::Value) -> (MockServer, ArchiveClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let client = ArchiveClient::with_config(ClientConfig {
        graphql_uri: server.uri(),
        retries: 1,
        retry_delay: Duration::from_millis(1),
        timeout: Duration::from_secs(5),
    });
    (server, client)
}

/// A1 — `events: [EventOutput]!`, schema.graphql:239.
#[tokio::test]
async fn null_event_element_decodes_to_none() {
    let (_server, client) = server_returning(json!({ "data": { "events": [null] } })).await;

    let events = client
        .get_events(EventFilterOptionsInput::for_address("B62q..."))
        .await
        .expect("a null element is legal and must decode");

    assert_eq!(events.len(), 1, "the null is kept, not elided");
    assert!(events[0].is_none());
}

/// A2 — `actions: [ActionOutput]!`, schema.graphql:240.
#[tokio::test]
async fn null_action_element_decodes_to_none() {
    let (_server, client) = server_returning(json!({ "data": { "actions": [null] } })).await;

    let actions = client
        .get_actions(ActionFilterOptionsInput::for_address("B62q..."))
        .await
        .expect("a null element is legal and must decode");

    assert_eq!(actions.len(), 1);
    assert!(actions[0].is_none());
}

/// A3 — `blocks: [Block]!`, schema.graphql:245.
#[tokio::test]
async fn null_block_element_decodes_to_none() {
    let (_server, client) = server_returning(json!({
        "data": {
            "blocks": [
                null,
                {
                    "blockHeight": 42,
                    "creator": "B62q",
                    "stateHash": "sh",
                    "parentHash": "ph",
                    "dateTime": "2023-08-14T22:30:01.000Z",
                    "transactions": {
                        "coinbase": "720000000000",
                        "userCommands": [],
                        "zkappCommands": [],
                        "feeTransfer": []
                    }
                }
            ]
        }
    }))
    .await;

    let blocks = client
        .get_blocks(GetBlocksOptions::default())
        .await
        .expect("a null element is legal and must decode");

    assert_eq!(blocks.len(), 2);
    assert!(blocks[0].is_none(), "a null block is None, not height 0");
    assert_eq!(
        blocks[1]
            .as_ref()
            .expect("second element is present")
            .block_height,
        42
    );
}

/// A4 — `EventData.data: [String]!`, schema.graphql:85.
#[tokio::test]
async fn null_event_data_member_decodes_to_none() {
    let (_server, client) = server_returning(json!({
        "data": {
            "events": [{
                "blockInfo": null,
                "eventData": [{
                    "accountUpdateId": "1",
                    "transactionInfo": null,
                    "data": ["0x1", null, ""]
                }]
            }]
        }
    }))
    .await;

    let events = client
        .get_events(EventFilterOptionsInput::for_address("B62q..."))
        .await
        .expect("a null data member is legal and must decode");

    let event = events[0].as_ref().expect("element is present");
    let data = &event.event_data.as_ref().expect("eventData present")[0]
        .as_ref()
        .expect("eventData element present")
        .data;

    assert_eq!(data.len(), 3);
    assert_eq!(data[0].as_deref(), Some("0x1"));
    assert_eq!(data[1], None, "a null member stays null");
    assert_eq!(
        data[2].as_deref(),
        Some(""),
        "an empty string stays distinguishable from null"
    );
}

/// A5 — `ActionData.data: [String]!`, schema.graphql:91.
#[tokio::test]
async fn null_action_data_member_decodes_to_none() {
    let (_server, client) = server_returning(json!({
        "data": {
            "actions": [{
                "blockInfo": null,
                "transactionInfo": null,
                "actionData": [{
                    "accountUpdateId": "1",
                    "transactionInfo": null,
                    "data": [null]
                }],
                "actionState": {
                    "actionStateOne": null,
                    "actionStateTwo": null,
                    "actionStateThree": null,
                    "actionStateFour": null,
                    "actionStateFive": null
                }
            }]
        }
    }))
    .await;

    let actions = client
        .get_actions(ActionFilterOptionsInput::for_address("B62q..."))
        .await
        .expect("a null data member is legal and must decode");

    let action = actions[0].as_ref().expect("element is present");
    let data = &action.action_data.as_ref().expect("actionData present")[0]
        .as_ref()
        .expect("actionData element present")
        .data;

    assert_eq!(data, &vec![None]);
}

/// A6 — `TransactionInfo.zkappAccountUpdateIds: [Int]!`, schema.graphql:117.
#[tokio::test]
async fn null_zkapp_account_update_id_decodes_to_none() {
    let (_server, client) = server_returning(json!({
        "data": {
            "events": [{
                "blockInfo": null,
                "eventData": [{
                    "accountUpdateId": "1",
                    "transactionInfo": {
                        "status": "applied",
                        "hash": "h",
                        "memo": "m",
                        "authorizationKind": "Proof",
                        "sequenceNumber": 0,
                        "zkappAccountUpdateIds": [42, null]
                    },
                    "data": ["0x1"]
                }]
            }]
        }
    }))
    .await;

    let events = client
        .get_events(EventFilterOptionsInput::for_address("B62q..."))
        .await
        .expect("a null id member is legal and must decode");

    let event = events[0].as_ref().expect("element is present");
    let info = event.event_data.as_ref().expect("eventData present")[0]
        .as_ref()
        .expect("eventData element present")
        .transaction_info
        .as_ref()
        .expect("transactionInfo present");

    assert_eq!(info.zkapp_account_update_ids, vec![Some(42), None]);
}

/// The whole SDL-legal response from the issue, in one call: nulls at every
/// level at once, which is how a real server would serve it.
#[tokio::test]
async fn every_nullable_position_at_once() {
    let (_server, client) = server_returning(json!({
        "data": {
            "events": [
                null,
                {
                    "blockInfo": null,
                    "eventData": [
                        null,
                        {
                            "accountUpdateId": "1",
                            "transactionInfo": {
                                "status": "applied",
                                "hash": "h",
                                "memo": "m",
                                "authorizationKind": "Proof",
                                "sequenceNumber": 0,
                                "zkappAccountUpdateIds": [42, null]
                            },
                            "data": ["0x1", null]
                        }
                    ]
                }
            ]
        }
    }))
    .await;

    let events = client
        .get_events(EventFilterOptionsInput::for_address("B62q"))
        .await
        .expect("the response from issue #6 must decode");

    assert_eq!(events.len(), 2);
    assert!(events[0].is_none());

    let event = events[1].as_ref().expect("second element is present");
    let event_data = event.event_data.as_ref().expect("eventData present");
    assert_eq!(event_data.len(), 2);
    assert!(event_data[0].is_none());
}
