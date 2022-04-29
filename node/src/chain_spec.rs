use arkworks_setups::Curve;
use arkworks_setups::common::setup_params;
use common::{AccountId, AuraId, Signature};
use webb_runtime::{
	wasm_binary_unwrap, AssetRegistryConfig, AuraConfig, BalancesConfig, CouncilConfig,
	GenesisConfig, HasherBls381Config, HasherBn254Config, MerkleTreeBls381Config,
	MerkleTreeBn254Config, MixerBn254Config, ParachainStakingConfig, SudoConfig, SystemConfig,
	VerifierBls381Config, VerifierBn254Config, KUNITS, UNITS,
};
use webb_primitives::Balance;

use cumulus_primitives_core::ParaId;
use pallet_parachain_staking::{InflationInfo, Range};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill, Percent,
};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<webb_runtime::GenesisConfig, Extensions>;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type WebbChainSpec =
	sc_service::GenericChainSpec<webb_runtime::GenesisConfig, Extensions>;

/// Specialized `ChainSpec` for the shell parachain runtime.
pub type ShellChainSpec = sc_service::GenericChainSpec<shell_runtime::GenesisConfig, Extensions>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate a crypto pair from seed
pub fn get_pair_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain
/// in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	get_pair_from_seed::<AuraId>(seed)
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we
/// have just one key).
pub fn webb_session_keys(keys: AuraId) -> webb_runtime::SessionKeys {
	webb_runtime::SessionKeys { aura: keys }
}

pub fn get_shell_chain_spec(id: ParaId) -> ShellChainSpec {
	ShellChainSpec::from_genesis(
		"Shell Local Testnet",
		"shell_local_testnet",
		ChainType::Local,
		move || shell_testnet_genesis(id),
		vec![],
		None,
		None,
		None,
		None,
		Extensions { relay_chain: "westend".into(), para_id: id.into() },
	)
}

/// Helper function to convert hex hashes to bytes
pub fn reverse_hex_bytes(hex: &str) -> [u8; 32] {
	let mut bytes = [0u8; 32];
	hex::decode_to_slice(hex, &mut bytes as &mut [u8]);
	// reverses the bytes(turns it to little endian)
	bytes.reverse();

	bytes
}

pub fn webb_development_config(id: ParaId) -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				// initial collators.
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						get_collator_keys_from_seed("Alice"),
						100 * KUNITS,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_collator_keys_from_seed("Bob"),
						100 * KUNITS,
					),
				],
				// Nominations
				vec![],
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				id,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Ford ID
		None,
		// Properties
		None,
		Extensions { relay_chain: "kusama".into(), para_id: id.into() },
	))
}

pub fn webb_local_testnet_config(id: ParaId) -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				// initial collators candidates.
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						get_collator_keys_from_seed("Alice"),
						100 * KUNITS,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_collator_keys_from_seed("Bob"),
						100 * KUNITS,
					),
				],
				// Nominations
				vec![],
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				id,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Fork ID
		None,
		// Properties
		None,
		// Extensions
		Extensions { relay_chain: "rococo".into(), para_id: id.into() },
	))
}

pub const ENDOWMENT: u128 = UNITS * 4096_000;

pub fn webb_test_genesis_inflation_config(
	endowed_accounts: Vec<AccountId>,
) -> InflationInfo<Balance> {
	let total = endowed_accounts.len() as u128;
	let total_issuance = total * ENDOWMENT;
	let sixty_percent = Percent::from_percent(60) * total_issuance;

	InflationInfo {
		expect: Range {
			min: sixty_percent - (Percent::from_percent(10) * sixty_percent),
			// 60% of total issuance at a yearly inflation rate of 5%
			ideal: sixty_percent,
			max: sixty_percent + (Percent::from_percent(10) * sixty_percent),
		},
		annual: Range {
			min: Perbill::from_percent(4),
			ideal: Perbill::from_percent(5),
			max: Perbill::from_percent(5),
		},
		// 8766 rounds (hours) in a year
		round: Range {
			min: Perbill::from_parts(Perbill::from_percent(4).deconstruct() / 8766),
			ideal: Perbill::from_parts(Perbill::from_percent(5).deconstruct() / 8766),
			max: Perbill::from_parts(Perbill::from_percent(5).deconstruct() / 8766),
		},
	}
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	candidates: Vec<(AccountId, AuraId, Balance)>,
	nominations: Vec<(AccountId, AccountId, Balance)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> GenesisConfig {
	let curve_bn254 = Curve::Bn254;

	log::info!("Bn254 x5 w3 params");
	let bn254_x5_3_params = setup_params::<ark_bn254::Fr>(curve_bn254, 5, 3);

	log::info!("Verifier params");
	let verifier_params = {
		let vk_bytes =
			include_bytes!("../../protocol-substrate-fixtures/mixer/bn254/x5/verifying_key.bin");

		vk_bytes.to_vec()
	};

	let mut default_zero_root_index: Vec<u8> = vec![0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31];
	let mut default_zero_root_hashes= Vec::new();

	default_zero_root_hashes.push(reverse_hex_bytes("2fe54c60d3acabf3343a35b6eba15db4821b340f76e741e2249685ed4899af6c"));
	default_zero_root_hashes.push(reverse_hex_bytes("13e37f2d6cb86c78ccc1788607c2b199788c6bb0a615a21f2e7a8e88384222f8"));
	default_zero_root_hashes.push(reverse_hex_bytes("217126fa352c326896e8c2803eec8fd63ad50cf65edfef27a41a9e32dc622765"));
	default_zero_root_hashes.push(reverse_hex_bytes("0e28a61a9b3e91007d5a9e3ada18e1b24d6d230c618388ee5df34cacd7397eee"));
	default_zero_root_hashes.push(reverse_hex_bytes("27953447a6979839536badc5425ed15fadb0e292e9bc36f92f0aa5cfa5013587"));
	default_zero_root_hashes.push(reverse_hex_bytes("194191edbfb91d10f6a7afd315f33095410c7801c47175c2df6dc2cce0e3affc"));
	default_zero_root_hashes.push(reverse_hex_bytes("1733dece17d71190516dbaf1927936fa643dc7079fc0cc731de9d6845a47741f"));
	default_zero_root_hashes.push(reverse_hex_bytes("267855a7dc75db39d81d17f95d0a7aa572bf5ae19f4db0e84221d2b2ef999219"));
	default_zero_root_hashes.push(reverse_hex_bytes("1184e11836b4c36ad8238a340ecc0985eeba665327e33e9b0e3641027c27620d"));
	default_zero_root_hashes.push(reverse_hex_bytes("0702ab83a135d7f55350ab1bfaa90babd8fc1d2b3e6a7215381a7b2213d6c5ce"));
	default_zero_root_hashes.push(reverse_hex_bytes("2eecc0de814cfd8c57ce882babb2e30d1da56621aef7a47f3291cffeaec26ad7"));
	default_zero_root_hashes.push(reverse_hex_bytes("280bc02145c155d5833585b6c7b08501055157dd30ce005319621dc462d33b47"));
	default_zero_root_hashes.push(reverse_hex_bytes("045132221d1fa0a7f4aed8acd2cbec1e2189b7732ccb2ec272b9c60f0d5afc5b"));
	default_zero_root_hashes.push(reverse_hex_bytes("27f427ccbf58a44b1270abbe4eda6ba53bd6ac4d88cf1e00a13c4371ce71d366"));
	default_zero_root_hashes.push(reverse_hex_bytes("1617eaae5064f26e8f8a6493ae92bfded7fde71b65df1ca6d5dcec0df70b2cef"));
	default_zero_root_hashes.push(reverse_hex_bytes("20c6b400d0ea1b15435703c31c31ee63ad7ba5c8da66cec2796feacea575abca"));
	default_zero_root_hashes.push(reverse_hex_bytes("09589ddb438723f53a8e57bdada7c5f8ed67e8fece3889a73618732965645eec"));
	default_zero_root_hashes.push(reverse_hex_bytes("0064b6a738a5ff537db7b220f3394f0ecbd35bfd355c5425dc1166bf3236079b"));
	default_zero_root_hashes.push(reverse_hex_bytes("095de56281b1d5055e897c3574ff790d5ee81dbc5df784ad2d67795e557c9e9f"));
	default_zero_root_hashes.push(reverse_hex_bytes("11cf2e2887aa21963a6ec14289183efe4d4c60f14ecd3d6fe0beebdf855a9b63"));
	default_zero_root_hashes.push(reverse_hex_bytes("2b0f6fc0179fa65b6f73627c0e1e84c7374d2eaec44c9a48f2571393ea77bcbb"));
	default_zero_root_hashes.push(reverse_hex_bytes("16fdb637c2abf9c0f988dbf2fd64258c46fb6a273d537b2cf1603ea460b13279"));
	default_zero_root_hashes.push(reverse_hex_bytes("21bbd7e944f6124dad4c376df9cc12e7ca66e47dff703ff7cedb1a454edcf0ff"));
	default_zero_root_hashes.push(reverse_hex_bytes("2784f8220b1c963e468f590f137baaa1625b3b92a27ad9b6e84eb0d3454d9962"));
	default_zero_root_hashes.push(reverse_hex_bytes("16ace1a65b7534142f8cc1aad810b3d6a7a74ca905d9c275cb98ba57e509fc10"));
	default_zero_root_hashes.push(reverse_hex_bytes("2328068c6a8c24265124debd8fe10d3f29f0665ea725a65e3638f6192a96a013"));
	default_zero_root_hashes.push(reverse_hex_bytes("2ddb991be1f028022411b4c4d2c22043e5e751c120736f00adf54acab1c9ac14"));
	default_zero_root_hashes.push(reverse_hex_bytes("0113798410eaeb95056a464f70521eb58377c0155f2fe518a5594d38cc209cc0"));
	default_zero_root_hashes.push(reverse_hex_bytes("202d1ae61526f0d0d01ef80fb5d4055a7af45721024c2c24cffd6a3798f54d50"));
	default_zero_root_hashes.push(reverse_hex_bytes("23ab323453748129f2765f79615022f5bebd6f4096a796300aab049a60b0f187"));
	default_zero_root_hashes.push(reverse_hex_bytes("1f15585f8947e378bcf8bd918716799da909acdb944c57150b1eb4565fda8aa0"));
	default_zero_root_hashes.push(reverse_hex_bytes("1eb064b21055ac6a350cf41eb30e4ce2cb19680217df3a243617c2838185ad06"));

	let default_zero_root_hashes_map = default_zero_root_index
		.into_iter()
		.zip(default_zero_root_hashes.into_iter())
		.collect();


	log::info!("Genesis Config");
	GenesisConfig {
		system: webb_runtime::SystemConfig { code: wasm_binary_unwrap().to_vec() },
		asset_registry: AssetRegistryConfig {
			asset_names: vec![],
			native_asset_name: b"WEBB".to_vec(),
			native_existential_deposit: webb_runtime::constants::currency::EXISTENTIAL_DEPOSIT,
		},
		balances: webb_runtime::BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect(),
		},
		parachain_info: webb_runtime::ParachainInfoConfig { parachain_id: id },
		session: webb_runtime::SessionConfig {
			keys: candidates
				.iter()
				.cloned()
				.map(|(acc, aura, _)| {
					(
						acc.clone(),                 // account id
						acc.clone(),                 // validator id
						webb_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
		},
		hasher_bn_254: HasherBn254Config {
			parameters: Some(bn254_x5_3_params.to_bytes()),
			phantom: Default::default(),
		},
		hasher_bls_381: HasherBls381Config {
			parameters: None,
			phantom: Default::default(),
		},
		verifier_bn_254: VerifierBn254Config {
			parameters: Some(verifier_params),
			phantom: Default::default(),
		},
		verifier_bls_381: VerifierBls381Config { parameters: None, phantom: Default::default() },
		merkle_tree_bn_254: MerkleTreeBn254Config {
			phantom: Default::default(),
			default_hashes: None,
		},
		merkle_tree_bls_381: MerkleTreeBls381Config {
			phantom: Default::default(),
			default_hashes: None,
			default_zero_root_hashes: Some(default_zero_root_hashes_map)
		},
		mixer_bn_254: MixerBn254Config {
			mixers: vec![(0, 10 * UNITS), (0, 100 * UNITS), (0, 1000 * UNITS)],
		},
		council: CouncilConfig::default(),
		treasury: Default::default(),
		parachain_staking: ParachainStakingConfig {
			candidates: candidates
				.iter()
				.cloned()
				.map(|(account, _, bond)| (account, bond))
				.collect(),
			nominations,
			inflation_config: webb_test_genesis_inflation_config(endowed_accounts),
		},
	}
}

fn shell_testnet_genesis(parachain_id: ParaId) -> shell_runtime::GenesisConfig {
	shell_runtime::GenesisConfig {
		system: shell_runtime::SystemConfig {
			code: shell_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		parachain_info: shell_runtime::ParachainInfoConfig { parachain_id },
		parachain_system: Default::default(),
	}
}
