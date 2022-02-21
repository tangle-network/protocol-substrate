use subxt::{BasicError, ClientBuilder, DefaultConfig, DefaultExtra};

#[subxt::subxt(runtime_metadata_path = "metadata/webb_metadata.scale")]
pub mod webb_runtime {}

type WebbRuntimeApi = webb_runtime::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>;

pub async fn client() -> Result<WebbRuntimeApi, BasicError> {
	let client = ClientBuilder::new().build().await?;
	Ok(client.to_runtime_api())
}
