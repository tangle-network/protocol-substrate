use webb::substrate::{
    protocol_substrate_runtime::api::{
        runtime_types::{
            frame_support::storage::bounded_vec::BoundedVec,
            pallet_asset_registry::types::AssetDetails,
            pallet_mixer::types::MixerMetadata,
        },
        RuntimeApi,
    },
    subxt::{DefaultConfig, DefaultExtra, ClientBuilder, BasicError},
};

type WebbRuntimeApi =
    RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>;

const URL: &str = "ninja";
pub async fn client() -> Result<WebbRuntimeApi, BasicError> {
	let client = ClientBuilder::new()
		.set_url(URL)
		.build()
		.await?;
	Ok(client.to_runtime_api())
}

fn main() {
	println!("Hello, world!");
}
