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
mina-archive-sdk = "1.0"
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

### Dates and times

The schema carries **two different time encodings**, a few fields apart, and both
arrive as strings:

| Field | Encoding | Example |
| --- | --- | --- |
| `BlockInfo::timestamp` | Unix epoch **milliseconds**, decimal string | `"1692054601000"` |
| `Block::date_time` | ISO-8601 | `"2023-08-14T23:10:01.000Z"` |

`BlockInfo::timestamp` is a raw pass-through of the archive DB column. Do **not** feed
it to an RFC 3339 parser, and do not read it as seconds.

On input, `date_time_gte` / `date_time_lt` must be ISO-8601. The server coerces them
with JavaScript's `new Date(value).getTime()`, and a value it cannot parse becomes
`NaN`, which reaches SQL as the string `"NaN"` and **matches nothing without
erroring** — HTTP 200, empty list, no diagnostic anywhere:

```text
"2023-08-14T00:00:00Z"  -> 1691971200000    ok
"2023-08-14"            -> 1691971200000    ok
"14/08/2023"            -> NaN              silently returns zero rows
"Aug 14 2023"           -> 1691964000000    parses, but timezone-dependent
```

Use the typed constructors to make that unrepresentable:

```rust
use mina_archive_sdk::BlockQueryInput;

let query = BlockQueryInput::default()
    .date_time_gte_from_unix_ms(1_691_971_200_000)
    .date_time_lt_from_unix_ms(1_692_054_601_000);
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
    Err(Error::RateLimited { retry_after, .. }) => eprintln!("slow down: retry after {retry_after:?}"),
    Err(Error::UnexpectedStatus { status, .. }) => eprintln!("unexpected HTTP {status}"),
    Err(Error::Connection { attempts, .. }) => eprintln!("unreachable after {attempts} tries"),
    Err(Error::MissingField { field, .. }) => eprintln!("schema mismatch: {field}"),
    Err(e) => eprintln!("error: {e}"),
}
# Ok(())
# }
```

#### Partial results

A response can legally carry **both** `data` and `errors` — the root lists and most of
their fields are nullable, so the server nulls the field that failed and reports it
alongside the rows that succeeded.

The methods above are strict: they treat that as a failure. When you would rather keep
what did arrive, use the `*_with_errors` family, which returns a `Response<T>`:

```rust,no_run
# async fn example(client: &mina_archive_sdk::ArchiveClient) -> mina_archive_sdk::Result<()> {
use mina_archive_sdk::EventFilterOptionsInput;

let resp = client
    .get_events_with_errors(EventFilterOptionsInput::for_address("B62q..."))
    .await?;

if resp.is_partial() {
    eprintln!("partial result: {}", resp.messages());
}
for group in resp.data.unwrap_or_default() {
    // the rows that did arrive
    let _ = group;
}
# Ok(())
# }
```

`data: null` with errors is a total failure on both paths, not a partial one.

#### Rate limiting and HTTP status

Every GraphQL-level error from this API arrives as **HTTP 200** with a populated
`errors` array — `extensions.status` is a payload field, not the HTTP status. HTTP 429
is the only non-200 the API emits, which makes it unusually informative: it
unambiguously means "slow down", and it is the one case where retrying the identical
request is correct.

The SDK retries 429 automatically, waiting for the interval the server names in
`retry-after`. If the retries run out, `Error::RateLimited` carries `retry_after`,
`limit` and `remaining` so a caller can schedule its own back-off.

Any other non-2xx becomes `Error::UnexpectedStatus`, which names the status rather than
reporting a decode failure. That is what a URL ending in `/graphql` produces.

#### Contract error codes

The API attaches `extensions.code` to every domain error and keeps the message text
deliberately minimal, so the **code is the intended discriminator** — do not match on
English message strings:

| Code | Meaning |
| --- | --- |
| `BLOCK_RANGE_ERROR` | The requested range exceeds `BLOCK_RANGE_SIZE`. Narrow it; never retry unchanged. |
| `ACTION_STATE_NOT_FOUND` | The action state is not in the archive. |
| `ACTION_STATE_OUT_OF_RANGE` | The action state falls outside the requested range. |
| `RATE_LIMITED` | Too many requests. Back off and retry. |

```rust,no_run
use mina_archive_sdk::{codes, Error};

# fn example(err: &Error) {
if err.has_graphql_code(codes::BLOCK_RANGE_ERROR) {
    // Narrow the range and try again.
}
for code in err.graphql_codes() {
    eprintln!("code: {code}");
}
# }
```

An **empty** result from `graphql_codes()` means no code was sent, not that nothing went
wrong. The server runs with masked errors, so an unexpected failure arrives as a generic
message with no `extensions` at all.

## Examples

```sh
ARCHIVE_GRAPHQL_URI=https://archive.example/ cargo run --example network_state
```

See `examples/`:

- `events.rs` — query events for an address
- `actions.rs` — query actions for an address
- `blocks.rs` — get the latest canonical blocks with currency parsing
- `network_state.rs` — check archive sync state

## Nullable elements

`get_events`, `get_actions` and `get_blocks` are `[T]!` in the SDL: the list
itself is always present, but **every element is nullable**, and the server is
free to return `null` there indefinitely — under the upstream versioning policy
`T` → `T!` is the only safe direction, so a null element never becomes a
breaking change. The same holds for `EventData::data`, `ActionData::data` and
`TransactionInfo::zkapp_account_update_ids`, which are `[String]!` and `[Int]!`
with nullable members.

Those positions are therefore `Option`-wrapped. The alternative — a
`deserialize_with` that silently drops nulls — was rejected: it converts a
visible failure into silently missing data.

`get_verification_key_updates` is the exception: its SDL type is
`[VerificationKeyUpdate!]!`, elements included, so it returns
`Vec<VerificationKeyUpdate>` with no `Option`.

## Version compatibility

The crate exports the schema version it speaks:

```rust
use mina_archive_sdk::SCHEMA_VERSION; // "1.0" — the Archive-Node-API schema major.minor
```

**`SCHEMA_VERSION`, not the crate version, is the compatibility check.** The
crate version is plain semver about the SDK's own surface:

| Part | Meaning |
| --- | --- |
| **Major** | A breaking change to the SDK's API — whether the schema forced it or not. |
| **Minor** | Additive: a new query, a new option, a new helper. |
| **Patch** | Fixes, docs, dependencies. |

The two still move together in the common cases: a breaking schema change
breaks the SDK surface, so it takes a major, and a schema minor that adds a
query is an SDK minor. What separates them is an **SDK-only** breaking change,
which now has a home. 2.0.0 is exactly that — it wrapped six positions in
`Option` so the `null`s the 1.0 schema always permitted stop failing the whole
query, and it speaks the same `1.0` schema 1.0.x did.

The schema is additive within a major version, so a crate whose
`SCHEMA_VERSION` major matches the server keeps working against a newer server;
it simply cannot reach what was added after it.

Earlier releases followed a stricter rule in which the crate's major.minor
*was* the schema version. That rule left no position for a breaking SDK-only
fix, which is why it was amended in 2.0.0.

### Migrating from 1.x to 2.0

`get_events`, `get_actions` and `get_blocks` now return `Vec<Option<T>>`, and
`EventData::data`, `ActionData::data` and
`TransactionInfo::zkapp_account_update_ids` now hold `Option` members. This is
strictly a widening — 1.x did not return these `None`s, it returned
`Err(Error::Decode)` for the entire query and gave you no way to reach the
elements that did decode.

```rust,ignore
// 1.x
for group in events { /* ... */ }

// 2.0 — drop the nulls
for group in events.into_iter().flatten() { /* ... */ }

// 2.0 — or handle them
for group in events {
    match group {
        Some(group) => { /* ... */ }
        None => { /* the server returned a null element */ }
    }
}
```

`get_network_state` and `get_verification_key_updates` are unchanged —
`[VerificationKeyUpdate!]!` has non-nullable elements and was already correct.

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
