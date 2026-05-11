//! Check the archive node's sync state.

use mina_archive_sdk::ArchiveClient;

#[tokio::main]
async fn main() -> mina_archive_sdk::Result<()> {
    let uri = std::env::var("ARCHIVE_GRAPHQL_URI")
        .unwrap_or_else(|_| "http://localhost:8080/".to_string());
    let client = ArchiveClient::new(&uri);

    let state = client.get_network_state().await?;
    let max = match state.max_block_height {
        Some(m) => m,
        None => {
            eprintln!("archive returned no maxBlockHeight — is it synced?");
            std::process::exit(1);
        }
    };

    println!("canonical max: {}", max.canonical_max_block_height);
    println!("pending max:   {}", max.pending_max_block_height);
    println!(
        "gap:           {}",
        max.pending_max_block_height - max.canonical_max_block_height
    );
    Ok(())
}
