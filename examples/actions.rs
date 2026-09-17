//! Query archived actions for a zkApp account.

use mina_archive_sdk::{ActionFilterOptionsInput, ArchiveClient, BlockStatusFilter};

#[tokio::main]
async fn main() -> mina_archive_sdk::Result<()> {
    let uri = std::env::var("ARCHIVE_GRAPHQL_URI")
        .unwrap_or_else(|_| "http://localhost:8080/".to_string());
    let address = std::env::var("MINA_ADDRESS")
        .unwrap_or_else(|_| "B62qiaEMrWiYdK7LcJ2ScdMyG8LzUxi7yaw17XvBD34on7UKfhAkRML".to_string());

    let client = ArchiveClient::new(&uri);
    let actions = client
        .get_actions(
            ActionFilterOptionsInput::for_address(address).status(BlockStatusFilter::Canonical),
        )
        .await?;

    println!("got {} action group(s)", actions.len());
    // `actions` is `[ActionOutput]!` in the SDL: the list is always present, but
    // any element may be null, so guard each one. `.flatten()` drops the nulls;
    // match on the Option instead if you need to see them.
    for group in actions.into_iter().flatten().take(5) {
        let height = group
            .block_info
            .map(|b| b.height.to_string())
            .unwrap_or_else(|| "?".into());
        let count = group.action_data.map(|d| d.len()).unwrap_or(0);
        println!("  block {height}: {count} action(s)");
    }
    Ok(())
}
