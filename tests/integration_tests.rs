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
    ClientConfig, EventFilterOptionsInput, GetBlocksOptions, VerificationKeyUpdateFilterInput,
};

const FIXTURE_ADDRESS: &str = "B62qiaEMrWiYdK7LcJ2ScdMyG8LzUxi7yaw17XvBD34on7UKfhAkRML";

/// The single verification key in the upstream sample archive dump.
const FIXTURE_VERIFICATION_KEY_HASH: &str =
    "330109536550383627416201330124291596191867681867265169258470531313815097966";

fn client() -> ArchiveClient {
    let uri = std::env::var("ARCHIVE_GRAPHQL_URI")
        .expect("set ARCHIVE_GRAPHQL_URI to a running Archive-Node-API endpoint");
    ArchiveClient::with_config(
        ClientConfig::new(uri)
            .attempts(2)
            .retry_delay(Duration::from_secs(1))
            .timeout(Duration::from_secs(30)),
    )
}

#[tokio::test]
#[ignore = "requires a running Archive-Node-API server (ARCHIVE_GRAPHQL_URI)"]
async fn network_state_returns_max_heights() {
    // The upstream resolver used to crash when either the canonical or the
    // pending row was missing, and this test tolerated the resulting GraphQL
    // error. It now returns maxBlockHeight: null for an empty archive
    // (network-service.ts), so a GraphQL error here is a real failure.
    //
    // `None` stays legal — maxBlockHeight is nullable by design — so this
    // asserts null-or-sane rather than `.expect(...)`, which panicked against
    // a legally-empty archive while the SDK itself had correctly returned
    // Ok(None).
    let state = client()
        .get_network_state()
        .await
        .expect("network_state must not fail");

    if let Some(max) = state.max_block_height {
        assert!(max.canonical_max_block_height >= 0);
        assert!(max.pending_max_block_height >= 0);
        assert!(
            max.pending_max_block_height >= max.canonical_max_block_height,
            "canonical <= pending by definition"
        );
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
        .get_blocks(
            GetBlocksOptions::default()
                .query(BlockQueryInput {
                    canonical: Some(true),
                    ..Default::default()
                })
                .limit(3)
                .sort_by(BlockSortBy::Desc),
        )
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
