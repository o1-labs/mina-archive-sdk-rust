//! Integration tests against a live Archive-Node-API server seeded with the
//! upstream `tests/integration/fixtures/archive_db.sql` fixture.
//!
//! Each test is `#[ignore]` so `cargo test` stays fast on a developer laptop.
//! Run with: `cargo test --test integration_tests -- --ignored --nocapture`,
//! pointing `ARCHIVE_GRAPHQL_URI` at a running server. CI provisions this
//! environment in `.github/workflows/integration.yml`.

use std::time::Duration;

use mina_archive_sdk::{
    ActionFilterOptionsInput, ArchiveClient, BlockQueryInput, BlockSortBy, BlockStatusFilter,
    ClientConfig, Error, EventFilterOptionsInput, GetBlocksOptions,
    VerificationKeyUpdateFilterInput,
};

const FIXTURE_ADDRESS: &str = "B62qiaEMrWiYdK7LcJ2ScdMyG8LzUxi7yaw17XvBD34on7UKfhAkRML";

/// The single verification key in the upstream sample archive dump.
const FIXTURE_VERIFICATION_KEY_HASH: &str =
    "330109536550383627416201330124291596191867681867265169258470531313815097966";

fn client() -> ArchiveClient {
    let uri = std::env::var("ARCHIVE_GRAPHQL_URI")
        .expect("set ARCHIVE_GRAPHQL_URI to a running Archive-Node-API endpoint");
    ArchiveClient::with_config(ClientConfig {
        graphql_uri: uri,
        retries: 2,
        retry_delay: Duration::from_secs(1),
        timeout: Duration::from_secs(30),
    })
}

#[tokio::test]
#[ignore = "requires a running Archive-Node-API server (ARCHIVE_GRAPHQL_URI)"]
async fn network_state_returns_max_heights() {
    // NOTE: against the static archive_db.sql fixture, the upstream
    // network-service resolver crashes if either canonical or pending rows
    // are missing (see Archive-Node-API's `src/services/network-service/
    // network-service.ts`). Tolerate that GraphQL error; once upstream is
    // patched, drop the match arm and keep the strict assertion.
    match client().get_network_state().await {
        Ok(state) => {
            let max = state.max_block_height.expect("max_block_height present");
            assert!(max.canonical_max_block_height >= 0);
        }
        Err(Error::Graphql { messages, .. }) => {
            eprintln!(
                "network_state returned a GraphQL error (known upstream issue against fixture): {messages}"
            );
        }
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[tokio::test]
#[ignore = "requires a running Archive-Node-API server (ARCHIVE_GRAPHQL_URI)"]
async fn events_against_fixture_address() {
    let _events = client()
        .get_events(
            EventFilterOptionsInput::for_address(FIXTURE_ADDRESS)
                .status(BlockStatusFilter::Canonical),
        )
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a running Archive-Node-API server (ARCHIVE_GRAPHQL_URI)"]
async fn actions_against_fixture_address() {
    let _actions = client()
        .get_actions(
            ActionFilterOptionsInput::for_address(FIXTURE_ADDRESS)
                .status(BlockStatusFilter::Canonical),
        )
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a running Archive-Node-API server (ARCHIVE_GRAPHQL_URI)"]
async fn blocks_desc_returns_ordered_results() {
    let blocks = client()
        .get_blocks(GetBlocksOptions {
            query: Some(BlockQueryInput {
                canonical: Some(true),
                ..Default::default()
            }),
            limit: Some(3),
            sort_by: Some(BlockSortBy::Desc),
        })
        .await
        .unwrap();
    // `[Block]!` has nullable elements, so the ordering check needs two
    // non-null neighbours rather than two positions.
    let heights: Vec<i64> = blocks
        .iter()
        .flatten()
        .map(|block| block.block_height)
        .collect();
    if heights.len() >= 2 {
        assert!(heights[0] >= heights[1], "DESC sort honored");
    }
}

#[tokio::test]
#[ignore = "requires a running Archive-Node-API server (ARCHIVE_GRAPHQL_URI)"]
async fn verification_key_updates_against_fixture_key() {
    let updates = client()
        .get_verification_key_updates(VerificationKeyUpdateFilterInput::new(
            FIXTURE_VERIFICATION_KEY_HASH,
            1,
            1000,
        ))
        .await
        .unwrap();
    for update in &updates {
        assert_eq!(update.verification_key_hash, FIXTURE_VERIFICATION_KEY_HASH);
    }
}
