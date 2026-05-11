//! Unit tests for `ArchiveClient` using `wiremock` to fake the GraphQL endpoint.

use std::time::Duration;

use mina_archive_sdk::{
    ActionFilterOptionsInput, ArchiveClient, BlockQueryInput, BlockSortBy, BlockStatusFilter,
    ClientConfig, Error, EventFilterOptionsInput, GetBlocksOptions,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fast_client(uri: &str) -> ArchiveClient {
    ArchiveClient::with_config(ClientConfig {
        graphql_uri: uri.to_string(),
        retries: 3,
        retry_delay: Duration::from_millis(1),
        timeout: Duration::from_secs(5),
    })
}

#[tokio::test]
async fn get_events_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "events": [{
                    "blockInfo": {
                        "height": 100,
                        "stateHash": "sh",
                        "parentHash": "ph",
                        "ledgerHash": "lh",
                        "chainStatus": "canonical",
                        "timestamp": "0",
                        "globalSlotSinceHardfork": 0,
                        "globalSlotSinceGenesis": 0,
                        "distanceFromMaxBlockHeight": 1,
                    },
                    "eventData": [{
                        "accountUpdateId": "1",
                        "transactionInfo": null,
                        "data": ["0x1"],
                    }],
                }],
            },
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let events = client
        .get_events(EventFilterOptionsInput::for_address("B62q..."))
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].block_info.as_ref().unwrap().height, 100);
}

#[tokio::test]
async fn get_network_state_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "networkState": {
                    "maxBlockHeight": {
                        "canonicalMaxBlockHeight": 1000,
                        "pendingMaxBlockHeight": 1010,
                    },
                },
            },
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let state = client.get_network_state().await.unwrap();
    assert_eq!(
        state.max_block_height.unwrap().canonical_max_block_height,
        1000
    );
}

#[tokio::test]
async fn get_actions_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "actions": [{
                    "blockInfo": null,
                    "transactionInfo": null,
                    "actionData": [],
                    "actionState": {
                        "actionStateOne": "a",
                        "actionStateTwo": null,
                        "actionStateThree": null,
                        "actionStateFour": null,
                        "actionStateFive": null,
                    },
                }],
            },
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let actions = client
        .get_actions(ActionFilterOptionsInput::for_address("B62q..."))
        .await
        .unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0].action_state.action_state_one.as_deref(),
        Some("a")
    );
}

#[tokio::test]
async fn get_blocks_passes_filters() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "blocks": [] },
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let blocks = client
        .get_blocks(GetBlocksOptions {
            query: Some(BlockQueryInput {
                canonical: Some(true),
                ..Default::default()
            }),
            limit: Some(5),
            sort_by: Some(BlockSortBy::Desc),
        })
        .await
        .unwrap();
    assert!(blocks.is_empty());
}

#[tokio::test]
async fn graphql_error_is_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [{ "message": "bad input" }],
        })))
        .expect(1) // <-- must NOT retry
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client
        .get_events(EventFilterOptionsInput::for_address(""))
        .await
        .unwrap_err();
    match err {
        Error::Graphql { messages, .. } => assert!(messages.contains("bad input")),
        other => panic!("expected Error::Graphql, got {other:?}"),
    }
}

#[tokio::test]
async fn transient_500_then_success() {
    let server = MockServer::start().await;
    // First call: 500. wiremock matches in registration order.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "networkState": {
                    "maxBlockHeight": { "canonicalMaxBlockHeight": 1, "pendingMaxBlockHeight": 2 },
                },
            },
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let state = client.get_network_state().await.unwrap();
    assert_eq!(
        state.max_block_height.unwrap().canonical_max_block_height,
        1
    );
}

#[tokio::test]
async fn persistent_failure_gives_connection_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;

    let client = ArchiveClient::with_config(ClientConfig {
        graphql_uri: server.uri(),
        retries: 2,
        retry_delay: Duration::from_millis(1),
        timeout: Duration::from_secs(5),
    });
    let err = client.get_network_state().await.unwrap_err();
    match err {
        Error::Connection { attempts, .. } => assert_eq!(attempts, 2),
        other => panic!("expected Error::Connection, got {other:?}"),
    }
}

#[tokio::test]
async fn missing_data_field_yields_missing_field_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": {} })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client
        .get_events(EventFilterOptionsInput::for_address("B62q"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::MissingField { field, .. } if field == "events"));
}

#[tokio::test]
async fn event_filter_serializes_camel_case_status() {
    // Ensure the serialized variables use camelCase + SCREAMING_SNAKE enum.
    let input = EventFilterOptionsInput::for_address("B62q")
        .status(BlockStatusFilter::Canonical)
        .from(1)
        .to(10);
    let j = serde_json::to_value(&input).unwrap();
    assert_eq!(j["status"], "CANONICAL");
    assert_eq!(j["address"], "B62q");
    assert_eq!(j["from"], 1);
    assert_eq!(j["to"], 10);
    assert!(j.get("tokenId").is_none()); // skip_serializing_if
}

#[tokio::test]
async fn block_sort_by_serializes_correctly() {
    let v = serde_json::to_value(BlockSortBy::Desc).unwrap();
    assert_eq!(v, json!("BLOCKHEIGHT_DESC"));
}

#[tokio::test]
async fn custom_query_builder_threads_variables() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "foo": 42 },
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let data = client
        .query("query Foo($x: Int) { foo(x: $x) }")
        .variables(json!({ "x": 7 }))
        .name("foo")
        .send()
        .await
        .unwrap();
    assert_eq!(data["foo"], 42);
}

#[test]
#[should_panic(expected = "retries must be at least 1")]
fn rejects_retries_zero() {
    ArchiveClient::with_config(ClientConfig {
        graphql_uri: "http://x".into(),
        retries: 0,
        retry_delay: Duration::from_secs(1),
        timeout: Duration::from_secs(1),
    });
}
