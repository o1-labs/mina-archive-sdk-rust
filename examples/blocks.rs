//! Query a range of blocks; demonstrates the Currency helper for parsing
//! coinbase / fee amounts.

use mina_archive_sdk::{ArchiveClient, BlockQueryInput, BlockSortBy, Currency, GetBlocksOptions};

#[tokio::main]
async fn main() -> mina_archive_sdk::Result<()> {
    let uri = std::env::var("ARCHIVE_GRAPHQL_URI")
        .unwrap_or_else(|_| "http://localhost:8080/".to_string());
    let client = ArchiveClient::new(&uri);

    let blocks = client
        .get_blocks(GetBlocksOptions {
            query: Some(BlockQueryInput {
                canonical: Some(true),
                ..Default::default()
            }),
            limit: Some(5),
            sort_by: Some(BlockSortBy::Desc),
        })
        .await?;

    println!("got {} block(s)", blocks.len());
    for block in blocks {
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
