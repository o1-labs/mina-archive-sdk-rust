//! Query a range of blocks; demonstrates the Currency helper for parsing
//! coinbase / fee amounts.
//!
//! The user-command count printed below is `0` against a stock server, and
//! that is not a bug in this example. Transaction detail requires the server
//! to set `ENABLE_BLOCK_TRANSACTION_DETAILS=true`, which defaults to `false`;
//! without it `parent_hash` is `""` and `user_commands`, `zkapp_commands` and
//! `fee_transfer` are empty. `coinbase` is populated either way.

use mina_archive_sdk::{ArchiveClient, BlockQueryInput, BlockSortBy, Currency, GetBlocksOptions};

#[tokio::main]
async fn main() -> mina_archive_sdk::Result<()> {
    let uri = std::env::var("ARCHIVE_GRAPHQL_URI")
        .unwrap_or_else(|_| "http://localhost:8080/".to_string());
    let client = ArchiveClient::new(&uri);

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
        .await?;

    println!("got {} block(s)", blocks.len());
    // `blocks` is `[Block]!` in the SDL: the list is always present, but any
    // element may be null, so guard each one. `.flatten()` drops the nulls;
    // match on the Option instead if you need to see them.
    for block in blocks.into_iter().flatten() {
        let coinbase = Currency::from_graphql(&block.transactions.coinbase)?;
        let prefix: String = block.creator.chars().take(12).collect();
        println!(
            "  block {} by {}…  coinbase={} MINA  ({} user commands)",
            block.block_height,
            prefix,
            coinbase,
            block.transactions.user_commands.len()
        );
    }
    Ok(())
}
