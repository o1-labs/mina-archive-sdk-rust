# mina-archive-sdk

[![CI](https://github.com/o1-labs/mina-archive-sdk-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/o1-labs/mina-archive-sdk-rust/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/mina-archive-sdk.svg?logo=rust)](https://crates.io/crates/mina-archive-sdk)
[![docs.rs](https://img.shields.io/docsrs/mina-archive-sdk?logo=docsdotrs)](https://docs.rs/mina-archive-sdk)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](./LICENSE)

Rust SDK for [Mina Protocol's Archive Node](https://github.com/o1-labs/Archive-Node-API) GraphQL endpoint.

Companion to the daemon-targeting [`mina-sdk`](https://crates.io/crates/mina-sdk) crate. This crate targets the separate **archive** endpoint defined by `o1-labs/Archive-Node-API` (events, actions, blocks, network state).

## Install

```toml
[dependencies]
mina-archive-sdk = "0.0.1"
tokio = { version = "1", features = ["full"] }
```

Tested on Rust stable.

## Quick start

```rust,no_run
use mina_archive_sdk::{ArchiveClient, BlockStatusFilter, EventFilterOptionsInput};

#[tokio::main]
async fn main() -> mina_archive_sdk::Result<()> {
    let client = ArchiveClient::new("https://archive.example/");

    let events = client.get_events(
        EventFilterOptionsInput::for_address("B62q...")
            .status(BlockStatusFilter::Canonical)
            .from(100)
            .to(200),
    ).await?;

    for group in events {
        let height = group.block_info.map(|b| b.height).unwrap_or(-1);
        let count = group.event_data.map(|d| d.len()).unwrap_or(0);
        println!("block {height}: {count} event(s)");
    }
    Ok(())
}
```

> **The endpoint is the root path.** Archive-Node-API serves GraphQL at `/`, not
> `/graphql`. Pass the base URL as-is — the SDK never appends a path, so a URL
> ending in `/graphql` reaches a route the server does not serve and returns 404.

## API

Each method on `ArchiveClient` maps 1:1 to a GraphQL query in the [Archive-Node-API schema](./schema.graphql):

| Method | Returns | Description |
| --- | --- | --- |
| `get_events(input)` | `Vec<EventOutput>` | Events emitted by a zkApp account. |
| `get_actions(input)` | `Vec<ActionOutput>` | Actions dispatched from a zkApp account. |
| `get_network_state()` | `NetworkStateOutput` | Archive's max canonical / pending block heights. |
| `get_blocks(opts)` | `Vec<Block>` | Blocks filtered by height/date range and chain status. Transaction detail needs `ENABLE_BLOCK_TRANSACTION_DETAILS` on the server — see below. |
| `get_verification_key_updates(input)` | `Vec<VerificationKeyUpdate>` | Applied account updates that set a given verification key, within a required block range. |
| `query(gql)` | builder | Arbitrary GraphQL through the same retry path. |
| `execute_query(gql, vars, name)` | `serde_json::Value` | Low-level escape hatch returning the raw `data` field. |

### Block transaction detail

`get_blocks` returns transaction detail only when the server sets
`ENABLE_BLOCK_TRANSACTION_DETAILS=true`. It **defaults to `false`**, and on a stock
server every block comes back with `parent_hash` as `""` and `user_commands`,
`zkapp_commands` and `fee_transfer` all empty. `coinbase` **is** populated either way,
which is what makes the response look healthy rather than obviously truncated.

### Configuration

```rust,no_run
use std::time::Duration;
use mina_archive_sdk::{ArchiveClient, ClientConfig};

let client = ArchiveClient::with_config(ClientConfig {
    graphql_uri: "https://archive.example/".to_string(),
    retries: 5,
    retry_delay: Duration::from_secs(10),
    timeout: Duration::from_secs(60),
});
```

### Currency

`Currency` wraps nanomina amounts in a `u64` for safe parsing of coinbase / fee / user-command values:

```rust
use mina_archive_sdk::Currency;

let coinbase = Currency::from_graphql("720000000000").unwrap();
assert_eq!(coinbase.mina(), "720.000000000");

let fee = Currency::from_mina("0.01").unwrap();
let total = coinbase + fee;
```

### Errors

All fallible operations return `Result<T>` over the `Error` enum. Match on variants:

```rust,no_run
use mina_archive_sdk::{ArchiveClient, Error, EventFilterOptionsInput};

# async fn example() -> mina_archive_sdk::Result<()> {
let client = ArchiveClient::new("https://archive.example/");
match client.get_events(EventFilterOptionsInput::for_address("B62q...")).await {
    Ok(events) => println!("{} groups", events.len()),
    Err(Error::Graphql { messages, .. }) => eprintln!("server rejected: {messages}"),
    Err(Error::Connection { attempts, .. }) => eprintln!("unreachable after {attempts} tries"),
    Err(Error::MissingField { field, .. }) => eprintln!("schema mismatch: {field}"),
    Err(e) => eprintln!("error: {e}"),
}
# Ok(())
# }
```

## Examples

```sh
ARCHIVE_GRAPHQL_URI=https://archive.example/ cargo run --example network_state
```

See `examples/`:

- `events.rs` — query events for an address
- `actions.rs` — query actions for an address
- `blocks.rs` — get the latest canonical blocks with currency parsing
- `network_state.rs` — check archive sync state

## Version compatibility

This SDK versions in lockstep with the [Archive-Node-API](https://github.com/o1-labs/Archive-Node-API) schema it speaks.

| Part | Meaning |
| --- | --- |
| **Major** | The schema major version. A breaking schema change moves both. |
| **Minor** | The schema minor version. A new query or argument moves both. |
| **Patch** | SDK-only changes — fixes, docs, dependencies. Independent of the server. |

So an SDK on `1.0.x` speaks the `1.0.x` schema, and matching the first two numbers is the whole compatibility check. The schema is additive within a major version, so an older SDK keeps working against a newer server; it simply cannot reach what was added after it.

## Development

```sh
cargo build
cargo test --lib --tests       # unit + wiremock tests, no infra needed
cargo fmt --check
cargo clippy --all-targets
```

Integration tests are `#[ignore]`d by default — they require a running Archive-Node-API server. CI provisions one in `.github/workflows/integration.yml`. To run locally:

```sh
ARCHIVE_GRAPHQL_URI=http://localhost:8080/ \
  cargo test --test integration_tests -- --ignored --nocapture
```

## Schema sync

`schema.graphql` is vendored from `o1-labs/Archive-Node-API@main`. The `Schema Drift` CI workflow compares them weekly and on PR; on drift, update both `schema.graphql` and `src/types.rs` in the same PR.

## License

Apache-2.0 — see [`LICENSE`](./LICENSE).
