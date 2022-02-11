use subxt::{DefaultConfig, DefaultExtra, ClientBuilder, BasicError};

#[subxt::subxt(runtime_metadata_path = "metadata/webb_metadata.scale")]
pub mod webb_runtime {}

type WebbRuntimeApi = webb_runtime:: RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>;

const URL: &str = "http://localhost:9933/";
pub async fn client() -> Result<WebbRuntimeApi, BasicError> {
	let client = ClientBuilder::new()
		.set_url(URL)
		.build()
		.await?;
	Ok(client.to_runtime_api())
}

#[async_std::main]
async fn main() {
	let api = client().await.unwrap();
    let block_number = 1;

    let block_hash = api
        .client
        .rpc()
        .block_hash(Some(block_number.into()))
        .await
        .unwrap();

    if let Some(hash) = block_hash {
        println!("Block hash for block number {}: {}", block_number, hash);
    } else {
        println!("Block number {} not found.", block_number);
    }
}
