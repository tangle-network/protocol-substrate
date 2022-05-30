use subxt::{BasicError, ClientBuilder, DefaultConfig, PolkadotExtrinsicParams};

#[subxt::subxt(runtime_metadata_path = "metadata/protocol_substrate_runtime.scale")]
pub mod webb_runtime {}

pub type WebbRuntimeApi =
	webb_runtime::RuntimeApi<DefaultConfig, PolkadotExtrinsicParams<DefaultConfig>>;

pub async fn client() -> Result<WebbRuntimeApi, BasicError> {
	let client = ClientBuilder::new().build().await?;
	Ok(client.to_runtime_api())
}
