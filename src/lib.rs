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
//! for group in events {
//!     let height = group.block_info.map(|b| b.height).unwrap_or(-1);
//!     let count = group.event_data.map(|d| d.len()).unwrap_or(0);
//!     println!("block {height}: {count} event(s)");
//! }
//! # Ok(())
//! # }
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
mod types;

pub use client::{ArchiveClient, ClientConfig, GetBlocksOptions, QueryBuilder};
pub use currency::Currency;
pub use error::{Error, GraphqlErrorEntry, Result};
pub use types::{
    ActionData, ActionFilterOptionsInput, ActionOutput, ActionStates, Block, BlockInfo,
    BlockQueryInput, BlockSortBy, BlockStatusFilter, BlockTransactions, EventData,
    EventFilterOptionsInput, EventOutput, FeeTransfer, MaxBlockHeightInfo, NetworkStateOutput,
    TransactionInfo, UserCommand, VerificationKeyUpdate, VerificationKeyUpdateFilterInput,
    ZkAppCommand,
};
