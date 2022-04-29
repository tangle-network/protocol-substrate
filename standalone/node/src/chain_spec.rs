use arkworks_setups::{common::setup_params, Curve};
use common::{AccountId, BabeId, Balance, Signature};
use std::collections::HashMap;

use itertools::Itertools;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};
use webb_runtime::{
	constants::currency::*, wasm_binary_unwrap, AnchorBn254Config, AnchorVerifierBn254Config,
	AssetRegistryConfig, AuthorityDiscoveryConfig, BabeConfig, Block, CouncilConfig,
	DemocracyConfig, ElectionsConfig, Element, GenesisConfig, GrandpaConfig, HasherBn254Config,
	ImOnlineConfig, IndicesConfig, MerkleTreeBn254Config, MixerBn254Config,
	MixerVerifierBn254Config, SessionConfig, StakerStatus, StakingConfig, SudoConfig,
	VAnchorBn254Config, VAnchorVerifier2x2Bn254Config,
};

// ImOnline consensus authority.
pub type ImOnlineId = pallet_im_online::sr25519::AuthorityId;

// AuthorityDiscovery consensus authority.
pub type AuthorityDiscoveryId = sp_authority_discovery::AuthorityId;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<webb_runtime::GenesisConfig, Extensions>;

#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
	seed: &str,
) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
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

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we
/// have just one key).
fn webb_session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> webb_runtime::SessionKeys {
	webb_runtime::SessionKeys { grandpa, babe, im_online, authority_discovery }
}

pub fn webb_development_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				vec![
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
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
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
		Default::default(),
	))
}

pub fn webb_local_testnet_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				vec![
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
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
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
		Default::default(),
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	mut initial_nominators: Vec<AccountId>,
	endowed_accounts: Vec<AccountId>,
	root_key: AccountId,
) -> GenesisConfig {
	let curve_bn254 = Curve::Bn254;

	log::info!("Bn254 x5 w3 params");
	let bn254_x5_3_params = setup_params::<ark_bn254::Fr>(curve_bn254, 5, 3);

	log::info!("Verifier params for mixer");
	let mixer_verifier_bn254_params = {
		let vk_bytes =
			include_bytes!("../../../protocol-substrate-fixtures/mixer/bn254/x5/verifying_key.bin");
		vk_bytes.to_vec()
	};

	log::info!("Verifier params for anchor");
	let anchor_verifier_bn254_params = {
		let vk_bytes = include_bytes!(
			"../../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/2/verifying_key.bin"
		);
		vk_bytes.to_vec()
	};

	log::info!("Verifier params for vanchor");
	let vanchor_verifier_bn254_params = {
		let vk_bytes = include_bytes!(
			"../../../protocol-substrate-fixtures/vanchor/bn254/x5/2-2-2/verifying_key.bin"
		);
		vk_bytes.to_vec()
	};

	let mut endowed_accounts: Vec<AccountId> = endowed_accounts;
	// endow all authorities and nominators.
	initial_authorities
		.iter()
		.map(|x| &x.0)
		.chain(initial_nominators.iter())
		.for_each(|x| {
			if !endowed_accounts.contains(x) {
				endowed_accounts.push(x.clone())
			}
		});

	let mut unique = vec![];
	unique.append(&mut endowed_accounts);
	unique.append(&mut initial_nominators);
	unique.append(&mut initial_authorities.iter().map(|x| &x.0).cloned().collect::<Vec<_>>());
	unique = unique.into_iter().unique().into_iter().collect::<Vec<_>>();

	// stakers: all validators and nominators.
	let mut _rng = rand::thread_rng();

	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	const STASH: Balance = ENDOWMENT / 1000;

	let mut default_zero_root_index: Vec<u8> = vec![
		0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
		25, 26, 27, 28, 29, 30, 31,
	];
	let mut default_zero_root_hashes = Vec::new();

	default_zero_root_hashes.push(reverse_hex_bytes(
		"2fe54c60d3acabf3343a35b6eba15db4821b340f76e741e2249685ed4899af6c",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"13e37f2d6cb86c78ccc1788607c2b199788c6bb0a615a21f2e7a8e88384222f8",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"217126fa352c326896e8c2803eec8fd63ad50cf65edfef27a41a9e32dc622765",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"0e28a61a9b3e91007d5a9e3ada18e1b24d6d230c618388ee5df34cacd7397eee",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"27953447a6979839536badc5425ed15fadb0e292e9bc36f92f0aa5cfa5013587",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"194191edbfb91d10f6a7afd315f33095410c7801c47175c2df6dc2cce0e3affc",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"1733dece17d71190516dbaf1927936fa643dc7079fc0cc731de9d6845a47741f",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"267855a7dc75db39d81d17f95d0a7aa572bf5ae19f4db0e84221d2b2ef999219",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"1184e11836b4c36ad8238a340ecc0985eeba665327e33e9b0e3641027c27620d",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"0702ab83a135d7f55350ab1bfaa90babd8fc1d2b3e6a7215381a7b2213d6c5ce",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"2eecc0de814cfd8c57ce882babb2e30d1da56621aef7a47f3291cffeaec26ad7",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"280bc02145c155d5833585b6c7b08501055157dd30ce005319621dc462d33b47",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"045132221d1fa0a7f4aed8acd2cbec1e2189b7732ccb2ec272b9c60f0d5afc5b",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"27f427ccbf58a44b1270abbe4eda6ba53bd6ac4d88cf1e00a13c4371ce71d366",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"1617eaae5064f26e8f8a6493ae92bfded7fde71b65df1ca6d5dcec0df70b2cef",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"20c6b400d0ea1b15435703c31c31ee63ad7ba5c8da66cec2796feacea575abca",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"09589ddb438723f53a8e57bdada7c5f8ed67e8fece3889a73618732965645eec",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"0064b6a738a5ff537db7b220f3394f0ecbd35bfd355c5425dc1166bf3236079b",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"095de56281b1d5055e897c3574ff790d5ee81dbc5df784ad2d67795e557c9e9f",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"11cf2e2887aa21963a6ec14289183efe4d4c60f14ecd3d6fe0beebdf855a9b63",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"2b0f6fc0179fa65b6f73627c0e1e84c7374d2eaec44c9a48f2571393ea77bcbb",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"16fdb637c2abf9c0f988dbf2fd64258c46fb6a273d537b2cf1603ea460b13279",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"21bbd7e944f6124dad4c376df9cc12e7ca66e47dff703ff7cedb1a454edcf0ff",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"2784f8220b1c963e468f590f137baaa1625b3b92a27ad9b6e84eb0d3454d9962",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"16ace1a65b7534142f8cc1aad810b3d6a7a74ca905d9c275cb98ba57e509fc10",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"2328068c6a8c24265124debd8fe10d3f29f0665ea725a65e3638f6192a96a013",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"2ddb991be1f028022411b4c4d2c22043e5e751c120736f00adf54acab1c9ac14",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"0113798410eaeb95056a464f70521eb58377c0155f2fe518a5594d38cc209cc0",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"202d1ae61526f0d0d01ef80fb5d4055a7af45721024c2c24cffd6a3798f54d50",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"23ab323453748129f2765f79615022f5bebd6f4096a796300aab049a60b0f187",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"1f15585f8947e378bcf8bd918716799da909acdb944c57150b1eb4565fda8aa0",
	));
	default_zero_root_hashes.push(reverse_hex_bytes(
		"1eb064b21055ac6a350cf41eb30e4ce2cb19680217df3a243617c2838185ad06",
	));

	let default_zero_root_hashes_map = default_zero_root_index
		.into_iter()
		.zip(default_zero_root_hashes.into_iter())
		.collect();
	log::info!("Genesis Config");
	GenesisConfig {
		system: webb_runtime::SystemConfig { code: wasm_binary_unwrap().to_vec() },
		asset_registry: AssetRegistryConfig {
			asset_names: vec![(b"TEST".to_vec(), 1)],
			native_asset_name: b"WEBB".to_vec(),
			native_existential_deposit: webb_runtime::constants::currency::EXISTENTIAL_DEPOSIT,
		},
		tokens: webb_runtime::TokensConfig {
			balances: unique.iter().cloned().map(|k| (k, 1, ENDOWMENT)).collect(),
		},
		balances: webb_runtime::BalancesConfig {
			balances: unique.iter().cloned().map(|k| (k, ENDOWMENT)).collect(),
		},
		indices: IndicesConfig { indices: vec![] },
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						webb_session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		staking: StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
				.collect(),
			..Default::default()
		},
		democracy: DemocracyConfig::default(),
		elections: ElectionsConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		council: CouncilConfig::default(),
		sudo: SudoConfig { key: Some(root_key) },
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(webb_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		im_online: ImOnlineConfig { keys: vec![] },
		authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		grandpa: GrandpaConfig { authorities: vec![] },
		treasury: Default::default(),
		hasher_bn_254: HasherBn254Config {
			parameters: Some(bn254_x5_3_params.to_bytes()),
			phantom: Default::default(),
		},
		mixer_verifier_bn_254: MixerVerifierBn254Config {
			parameters: Some(mixer_verifier_bn254_params),
			phantom: Default::default(),
		},
		anchor_verifier_bn_254: AnchorVerifierBn254Config {
			parameters: Some(anchor_verifier_bn254_params),
			phantom: Default::default(),
		},
		v_anchor_verifier_2x_2_bn_254: VAnchorVerifier2x2Bn254Config {
			parameters: Some(vanchor_verifier_bn254_params),
			phantom: Default::default(),
		},
		merkle_tree_bn_254: MerkleTreeBn254Config {
			phantom: Default::default(),
			default_hashes: None,
			default_zero_root_hashes: Some(default_zero_root_hashes_map),
		},
		mixer_bn_254: MixerBn254Config {
			mixers: vec![
				(0, 10 * UNITS),
				(0, 100 * UNITS),
				(0, 1000 * UNITS),
				(1, 10 * UNITS),
				(1, 100 * UNITS),
			],
		},
		anchor_bn_254: AnchorBn254Config {
			anchors: vec![(0, 10 * UNITS, 2), (0, 100 * UNITS, 2), (0, 1000 * UNITS, 2)],
		},
		v_anchor_bn_254: VAnchorBn254Config {
			max_deposit_amount: 1_000_000 * UNITS,
			min_withdraw_amount: 0,
			vanchors: vec![(0, 2)],
			phantom: Default::default(),
		},
	}
}
