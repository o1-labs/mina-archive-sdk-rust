//! Unit tests for `ArchiveClient` using `wiremock` to fake the GraphQL endpoint.

use std::time::Duration;

use mina_archive_sdk::{
    codes, ActionFilterOptionsInput, ArchiveClient, BlockQueryInput, BlockSortBy,
    BlockStatusFilter, ClientConfig, Error, EventFilterOptionsInput, GetBlocksOptions,
    VerificationKeyUpdateFilterInput,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fast_client(uri: &str) -> ArchiveClient {
    ArchiveClient::with_config(
        ClientConfig::new(uri)
            .attempts(3)
            .retry_delay(Duration::from_millis(1))
            .timeout(Duration::from_secs(5)),
    )
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
    let event = events[0].as_ref().expect("element is not null");
    assert_eq!(event.block_info.as_ref().unwrap().height, 100);
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
    let action = actions[0].as_ref().expect("element is not null");
    assert_eq!(action.action_state.action_state_one.as_deref(), Some("a"));
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
        .get_blocks(
            GetBlocksOptions::default()
                .query(BlockQueryInput {
                    canonical: Some(true),
                    ..Default::default()
                })
                .limit(5)
                .sort_by(BlockSortBy::Desc),
        )
        .await
        .unwrap();
    assert!(blocks.is_empty());
}

#[tokio::test]
async fn get_verification_key_updates_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "verificationKeyUpdates": [{
                    "accountUpdateId": "42",
                    "address": "B62qtest",
                    "tokenId": "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf",
                    "verificationKeyHash": "3301095365503836274162013301242915961918676818672",
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
                    "transactionInfo": {
                        "status": "applied",
                        "hash": "txhash",
                        "memo": "",
                        "authorizationKind": "Proof",
                        "sequenceNumber": 0,
                        "zkappAccountUpdateIds": [42],
                    },
                }],
            },
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let updates = client
        .get_verification_key_updates(VerificationKeyUpdateFilterInput::new(
            "3301095365503836274162013301242915961918676818672",
            1,
            1000,
        ))
        .await
        .unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].address, "B62qtest");
    assert_eq!(updates[0].block_info.height, 100);
}

#[tokio::test]
async fn verification_key_filter_serializes_required_range() {
    let input =
        VerificationKeyUpdateFilterInput::new("vk", 10, 20).status(BlockStatusFilter::Canonical);
    let json = serde_json::to_value(&input).unwrap();
    assert_eq!(json["verificationKeyHash"], "vk");
    assert_eq!(json["from"], 10);
    assert_eq!(json["to"], 20);
    assert_eq!(json["status"], "CANONICAL");

    // `status` is the only optional member; it disappears when unset.
    let bare = serde_json::to_value(VerificationKeyUpdateFilterInput::new("vk", 1, 2)).unwrap();
    assert!(bare.get("status").is_none());
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

    let client = ArchiveClient::with_config(
        ClientConfig::new(server.uri())
            .attempts(2)
            .retry_delay(Duration::from_millis(1))
            .timeout(Duration::from_secs(5)),
    );
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
    // ClientConfig is #[non_exhaustive], so this cannot be a struct literal —
    // and ClientConfig::attempts raises 0 to 1 rather than building an invalid
    // config. Assigning the public field is the remaining way in, and
    // with_config must still reject it.
    let mut config = ClientConfig::new("http://x");
    config.retries = 0;
    ArchiveClient::with_config(config);
}

/// An empty `errors` array is not an error in GraphQL (#11). Any proxy or
/// gateway that normalises the envelope to always carry `"errors": []` used to
/// turn every successful call into a failure with an empty message.
#[tokio::test]
async fn empty_errors_array_is_not_a_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [],
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
    let state = client
        .get_network_state()
        .await
        .expect("an empty errors array must not fail the call");
    assert_eq!(
        state.max_block_height.unwrap().canonical_max_block_height,
        1000
    );
}

/// A non-empty `errors` array must still fail, so the fix above cannot be
/// mistaken for "ignore the errors key".
#[tokio::test]
async fn non_empty_errors_array_still_fails() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [{ "message": "Block range exceeds maximum" }],
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client.get_network_state().await.unwrap_err();
    assert!(
        err.to_string().contains("Block range exceeds maximum"),
        "unexpected error: {err}"
    );
}

/// The API attaches `extensions.code` to every domain error and keeps message
/// text deliberately minimal, so the code is the intended discriminator (#9).
/// All of it used to be discarded except the message string.
#[tokio::test]
async fn graphql_error_carries_extensions_path_and_locations() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [{
                "message": "Block range exceeds maximum",
                "path": ["events"],
                "locations": [{ "line": 2, "column": 3 }],
                "extensions": { "code": "BLOCK_RANGE_ERROR", "status": 400 },
            }],
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client
        .get_events(EventFilterOptionsInput::for_address("B62q..."))
        .await
        .unwrap_err();

    assert_eq!(err.graphql_codes(), vec![codes::BLOCK_RANGE_ERROR]);
    assert!(err.has_graphql_code(codes::BLOCK_RANGE_ERROR));
    assert!(!err.has_graphql_code(codes::RATE_LIMITED));

    match &err {
        mina_archive_sdk::Error::Graphql { errors, .. } => {
            let e = &errors[0];
            assert_eq!(e.code.as_deref(), Some("BLOCK_RANGE_ERROR"));
            assert_eq!(e.path.as_ref().unwrap()[0], json!("events"));
            assert_eq!(e.locations.as_ref().unwrap()[0]["line"], json!(2));
            // The whole extensions object is kept, not just the code.
            assert_eq!(e.extensions.as_ref().unwrap()["status"], json!(400));
        }
        other => panic!("expected Error::Graphql, got {other:?}"),
    }
}

/// The server runs `maskedErrors: { isDev: false }`, so an unexpected error
/// arrives with a generic message and NO extensions. `code` must stay optional
/// and this path must keep working.
#[tokio::test]
async fn masked_error_without_extensions_still_decodes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [{ "message": "Unexpected error." }],
            "data": null,
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client.get_network_state().await.unwrap_err();

    assert!(err.graphql_codes().is_empty(), "a masked error has no code");
    assert!(err.to_string().contains("Unexpected error."));
    match &err {
        mina_archive_sdk::Error::Graphql { errors, .. } => {
            assert!(errors[0].code.is_none());
            assert!(errors[0].extensions.is_none());
        }
        other => panic!("expected Error::Graphql, got {other:?}"),
    }
}

/// The rate limiter returns 429 with a GraphQL-shaped body plus three headers,
/// before GraphQL runs (#8). All of it used to be discarded: the error came
/// back indistinguishable from a query error, with no status and no retry hint.
#[tokio::test]
async fn rate_limited_exposes_retry_after_and_budget() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "30")
                .insert_header("x-ratelimit-limit", "600")
                .insert_header("x-ratelimit-remaining", "0")
                .set_body_json(json!({
                    "errors": [{
                        "message": "Too many requests",
                        "extensions": { "code": "RATE_LIMITED" },
                    }],
                })),
        )
        .mount(&server)
        .await;

    // retries = 1 so the call does not sleep through retry-after.
    let client = ArchiveClient::with_config(
        ClientConfig::new(server.uri())
            .attempts(1)
            .retry_delay(Duration::from_millis(0))
            .timeout(Duration::from_secs(5)),
    );
    let err = client.get_network_state().await.unwrap_err();

    match &err {
        mina_archive_sdk::Error::RateLimited {
            retry_after,
            limit,
            remaining,
            messages,
            ..
        } => {
            assert_eq!(*retry_after, Some(Duration::from_secs(30)));
            assert_eq!(*limit, Some(600));
            assert_eq!(*remaining, Some(0));
            assert!(messages.contains("Too many requests"));
        }
        other => panic!("expected Error::RateLimited, got {other:?}"),
    }
}

/// A 429 is the one status where retrying the identical request is correct,
/// and retry-after is the server telling us how long to wait.
#[tokio::test]
async fn rate_limited_is_retried_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_json(json!({ "errors": [{ "message": "Too many requests" }] })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "networkState": { "maxBlockHeight": {
                "canonicalMaxBlockHeight": 1000, "pendingMaxBlockHeight": 1010 } } },
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

/// `timeout` bounds one HTTP request; it never covered the sleep between
/// attempts, so a server asking for `retry-after: 3600` parked the call for an
/// hour whatever the timeout said. The ceiling turns that into an immediate
/// `RateLimited` carrying the requested delay, so the caller decides.
#[tokio::test]
async fn retry_after_beyond_the_ceiling_returns_at_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "3600")
                .set_body_json(json!({ "errors": [{ "message": "Too many requests" }] })),
        )
        .mount(&server)
        .await;

    // attempts(3) would retry, and retry_delay is irrelevant: what decides is
    // that the server's 3600s ask exceeds the 60s ceiling.
    let client = ArchiveClient::with_config(
        ClientConfig::new(server.uri())
            .attempts(3)
            .retry_delay(Duration::from_millis(0))
            .max_retry_after(Duration::from_secs(60))
            .timeout(Duration::from_secs(5)),
    );

    let started = std::time::Instant::now();
    let err = client.get_network_state().await.unwrap_err();
    let elapsed = started.elapsed();

    match &err {
        mina_archive_sdk::Error::RateLimited { retry_after, .. } => {
            assert_eq!(*retry_after, Some(Duration::from_secs(3600)));
        }
        other => panic!("expected Error::RateLimited, got {other:?}"),
    }
    // The point of the fix: it returned instead of sleeping.
    assert!(
        elapsed < Duration::from_secs(5),
        "call took {elapsed:?}; it slept on retry-after instead of returning"
    );
}

/// Within the ceiling, retry-after is still honoured and the call still
/// succeeds on the retry.
#[tokio::test]
async fn retry_after_within_the_ceiling_is_still_honoured() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_json(json!({ "errors": [{ "message": "Too many requests" }] })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "networkState": { "maxBlockHeight": {
                "canonicalMaxBlockHeight": 42, "pendingMaxBlockHeight": 43 } } },
        })))
        .mount(&server)
        .await;

    let client = ArchiveClient::with_config(
        ClientConfig::new(server.uri())
            .attempts(3)
            .retry_delay(Duration::from_millis(0))
            .max_retry_after(Duration::from_secs(60))
            .timeout(Duration::from_secs(5)),
    );
    let state = client.get_network_state().await.unwrap();
    assert_eq!(
        state.max_block_height.unwrap().canonical_max_block_height,
        42
    );
}

/// A 404 with a non-GraphQL body is the shape you get from POSTing to
/// /graphql. It used to surface as "error decoding response body" or as
/// "missing field", both of which misdirect the diagnosis.
#[tokio::test]
async fn non_graphql_404_reports_the_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client
        .get_events(EventFilterOptionsInput::for_address("B62q..."))
        .await
        .unwrap_err();

    let display = err.to_string();
    assert!(display.contains("404"), "should name the status: {display}");
    assert!(
        !display.contains("decoding response body"),
        "should not blame the body: {display}"
    );
    assert!(
        !display.contains("missing field"),
        "should not blame the schema: {display}"
    );
    assert!(matches!(
        err,
        mina_archive_sdk::Error::UnexpectedStatus { .. }
    ));
}

/// A 4xx that IS GraphQL-shaped keeps its errors array rather than being
/// flattened into UnexpectedStatus.
#[tokio::test]
async fn graphql_shaped_4xx_keeps_its_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "errors": [{
                "message": "Block range exceeds maximum",
                "extensions": { "code": "BLOCK_RANGE_ERROR" },
            }],
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client.get_network_state().await.unwrap_err();
    assert!(err.has_graphql_code("BLOCK_RANGE_ERROR"));
    match err {
        mina_archive_sdk::Error::Graphql { status, .. } => {
            assert_eq!(status.as_u16(), 400);
        }
        other => panic!("expected Error::Graphql, got {other:?}"),
    }
}

/// HTTP 200 with BOTH a partial `data` and an `errors` array is a normal
/// GraphQL outcome (#10). The strict path reported it as a total failure and
/// the rows that did succeed were unrecoverable.
#[tokio::test]
async fn partial_data_and_errors_are_both_reachable() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "events": [{
                "blockInfo": {
                    "height": 100, "stateHash": "3NK...", "parentHash": "3NL...",
                    "ledgerHash": "jx...", "chainStatus": "canonical",
                    "timestamp": "1692054601000", "globalSlotSinceHardfork": 1,
                    "globalSlotSinceGenesis": 2, "distanceFromMaxBlockHeight": 3,
                },
                "eventData": null,
            }]},
            "errors": [{ "message": "partial" }],
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let resp = client
        .get_events_with_errors(EventFilterOptionsInput::for_address("B62q..."))
        .await
        .expect("a partial response is not a transport failure");

    // Both halves, from the single call.
    let events = resp.data.as_ref().expect("the decoded event must survive");
    assert_eq!(events.len(), 1);
    let event = events[0].as_ref().expect("element is not null");
    assert_eq!(event.block_info.as_ref().unwrap().height, 100);
    assert_eq!(resp.errors.len(), 1);
    assert_eq!(resp.errors[0].message, "partial");
    assert!(resp.is_partial());
    assert_eq!(resp.messages(), "partial");
}

/// The strict methods must stay strict — a partial response is still an error
/// there, so this is opt-in rather than a silent change of meaning.
#[tokio::test]
async fn strict_method_still_fails_on_partial_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "events": [] },
            "errors": [{ "message": "partial" }],
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client
        .get_events(EventFilterOptionsInput::for_address("B62q..."))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("partial"), "got {err}");
}

/// data: null with errors is a total failure, not a partial one, on both paths.
#[tokio::test]
async fn null_data_with_errors_is_a_plain_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": null,
            "errors": [{ "message": "Unexpected error." }],
        })))
        .mount(&server)
        .await;

    let client = fast_client(&server.uri());
    let err = client.get_network_state_with_errors().await.unwrap_err();
    assert!(matches!(err, Error::Graphql { .. }), "got {err:?}");
}

/// `ArchiveClient` must be `Clone + Debug + Send + Sync`.
///
/// It was `Send + Sync` but neither `Clone` nor `Debug`, so a caller wanting a
/// client per task had to wrap it in an `Arc` for no reason — the inner
/// `reqwest::Client` is reference-counted already — and could not put it in a
/// `#[derive(Debug)]` application-state struct.
#[test]
fn client_is_clone_debug_send_sync() {
    fn assert<T: Clone + std::fmt::Debug + Send + Sync>() {}
    assert::<ArchiveClient>();

    let client = ArchiveClient::new("https://archive.example/");
    let clone = client.clone();
    assert!(
        !format!("{client:?}").is_empty(),
        "Debug must render something"
    );
    drop(clone);
}
