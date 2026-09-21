//! Rust SDK for [Mina Protocol's](https://minaprotocol.com) Archive Node
//! GraphQL endpoint defined by [`o1-labs/Archive-Node-API`](https://github.com/o1-labs/Archive-Node-API).
//!
//! Companion to the daemon-targeting [`mina-sdk`](https://crates.io/crates/mina-sdk)
//! crate. This crate targets the separate **archive** endpoint — events,
//! actions, blocks, network state.
//!
//! # Quick start
//!
//! Archive-Node-API serves GraphQL at the root path `/`, not `/graphql`. Pass
//! the base URL as-is — the SDK never appends a path, so a URL ending in
//! `/graphql` returns 404.
//!
//! ```no_run
//! # async fn example() -> mina_archive_sdk::Result<()> {
//! use mina_archive_sdk::{ArchiveClient, BlockStatusFilter, EventFilterOptionsInput};
//!
//! let client = ArchiveClient::new("https://archive.example/");
//! let events = client.get_events(
//!     EventFilterOptionsInput::for_address("B62q...")
//!         .status(BlockStatusFilter::Canonical)
//!         .from(100)
//!         .to(200),
//! ).await?;
//!
//! // `[EventOutput]!` has nullable elements, so each one arrives as an
//! // `Option`. `.flatten()` drops the nulls; match on it to see them.
//! for group in events.into_iter().flatten() {
//!     let height = group.block_info.map(|b| b.height).unwrap_or(-1);
//!     let count = group.event_data.map(|d| d.len()).unwrap_or(0);
//!     println!("block {height}: {count} event(s)");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Runtime requirements
//!
//! This crate requires a Tokio runtime **with the time driver enabled**.
//! `reqwest`'s per-request timeout needs it, so on a runtime built without
//! `enable_time()` every query panics with "A Tokio 1.x context was found, but
//! timers are disabled". `#[tokio::main]` and `#[tokio::test]` enable it;
//! a hand-built `Builder::new_current_thread()` does not unless you say so.
//!
//! ```no_run
//! let runtime = tokio::runtime::Builder::new_current_thread()
//!     .enable_time()
//!     .enable_io()
//!     .build()
//!     .unwrap();
//! # let _ = runtime;
//! ```
//!
//! # Currency
//!
//! Coinbase / fee / user-command amounts are returned as nanomina decimal
//! strings. Use [`Currency`] for safe parsing and arithmetic:
//!
//! ```
//! use mina_archive_sdk::Currency;
//!
//! let coinbase = Currency::from_graphql("720000000000").unwrap();
//! assert_eq!(coinbase.mina(), "720.000000000");
//!
//! let fee = Currency::from_mina("0.01").unwrap();
//! let total = coinbase + fee;
//! ```
//!
//! # Error handling
//!
//! All fallible operations return [`Result<T>`](Result), which uses [`Error`].
//! Match on variants:
//!
//! ```no_run
//! # async fn example() -> mina_archive_sdk::Result<()> {
//! use mina_archive_sdk::{ArchiveClient, EventFilterOptionsInput, Error};
//!
//! let client = ArchiveClient::new("https://archive.example/");
//! match client.get_events(EventFilterOptionsInput::for_address("B62q...")).await {
//!     Ok(events) => println!("got {} groups", events.len()),
//!     Err(Error::Graphql { messages, .. }) => eprintln!("server rejected the query: {messages}"),
//!     Err(Error::Connection { attempts, .. }) => eprintln!("archive unreachable after {attempts} tries"),
//!     Err(e) => eprintln!("error: {e}"),
//! }
//! # Ok(())
//! # }
//! ```

mod client;
mod currency;
pub mod error;
pub mod queries;
mod response;
mod types;

/// The Archive-Node-API schema version this crate speaks.
///
/// This constant — not the crate version — is the compatibility check. The
/// crate version is plain semver about the SDK's own surface, so an SDK-only
/// breaking change can take a major without claiming the schema moved.
///
/// The schema is additive within a major version, so a crate whose
/// `SCHEMA_VERSION` major matches the server keeps working against a newer
/// server; it simply cannot reach what was added after it.
pub const SCHEMA_VERSION: &str = "1.0";

/// The README's code fences, compiled as doctests.
///
/// `RUSTDOCFLAGS`/`RUSTFLAGS` never reached these: in-source examples are
/// covered by `cargo test --doc`, README fences are covered by nothing, which
/// is how the quick-start example went on not compiling. `#[cfg(doctest)]`
/// keeps the README out of the rendered crate docs while still building every
/// fence in it.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

pub use client::{ArchiveClient, ClientConfig, GetBlocksOptions, QueryBuilder};
pub use currency::Currency;
pub use error::{codes, Error, GraphqlErrorEntry, Result};
pub use response::Response;
pub use types::{
    ActionData, ActionFilterOptionsInput, ActionOutput, ActionStates, Block, BlockInfo,
    BlockQueryInput, BlockSortBy, BlockStatusFilter, BlockTransactions, EventData,
    EventFilterOptionsInput, EventOutput, FeeTransfer, MaxBlockHeightInfo, NetworkStateOutput,
    TransactionInfo, UserCommand, VerificationKeyUpdate, VerificationKeyUpdateFilterInput,
    ZkAppCommand,
};
