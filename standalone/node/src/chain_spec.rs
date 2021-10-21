use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	prelude::ark_bn254::Bn254,
	setup::{
		common::Curve,
		mixer::{setup_groth16_circuit_circomx5, setup_groth16_random_circuit_circomx5, setup_random_circuit_circomx5},
	},
	utils::{
		get_mds_poseidon_bls381_x3_5, get_mds_poseidon_bls381_x5_5, get_mds_poseidon_bn254_x3_5,
		get_mds_poseidon_bn254_x5_5, get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_bls381_x3_5,
		get_rounds_poseidon_bls381_x5_5, get_rounds_poseidon_bn254_x3_5, get_rounds_poseidon_bn254_x5_5,
		get_rounds_poseidon_circom_bn254_x5_3,
	},
};
use common::{AccountId, AuraId, BabeId, Balance, Signature};

use darkwebb_runtime::{
	constants::{currency::*, time::*},
	wasm_binary_unwrap, AuthorityDiscoveryConfig, BLS381Poseidon3x5HasherConfig, BLS381Poseidon5x5HasherConfig,
	BN254CircomPoseidon3x5HasherConfig, BN254Poseidon3x5HasherConfig, BN254Poseidon5x5HasherConfig, BabeConfig, Block,
	CouncilConfig, DemocracyConfig, ElectionsConfig, GenesisConfig, GrandpaConfig, ImOnlineConfig, IndicesConfig,
	MerkleTreeConfig, SessionConfig, StakerStatus, StakingConfig, SudoConfig, SystemConfig, VerifierConfig,
};
use itertools::Itertools;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

// ImOnline consensus authority.
pub type ImOnlineId = pallet_im_online::sr25519::AuthorityId;

// AuthorityDiscovery consensus authority.
pub type AuthorityDiscoveryId = sp_authority_discovery::AuthorityId;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<darkwebb_runtime::GenesisConfig, Extensions>;

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

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
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
) -> (
	AccountId,
	AccountId,
	GrandpaId,
	BabeId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we
/// have just one key).
fn darkwebb_session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> darkwebb_runtime::SessionKeys {
	darkwebb_runtime::SessionKeys {
		grandpa,
		babe,
		im_online,
		authority_discovery,
	}
}

pub fn darkwebb_development_config() -> Result<ChainSpec, String> {
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
				vec![],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		Default::default(),
	))
}

pub fn darkwebb_local_testnet_config() -> Result<ChainSpec, String> {
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
				vec![],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
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
	initial_nominators: Vec<AccountId>,
	endowed_accounts: Vec<AccountId>,
	root_key: AccountId,
) -> GenesisConfig {
	let circom_params = {
		let rounds = get_rounds_poseidon_circom_bn254_x5_3::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		let mds = get_mds_poseidon_circom_bn254_x5_3::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	let bls381_3x_5_params = {
		let rounds = get_rounds_poseidon_bls381_x3_5::<arkworks_gadgets::prelude::ark_bls12_381::Fr>();
		let mds = get_mds_poseidon_bls381_x3_5::<arkworks_gadgets::prelude::ark_bls12_381::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	let bls381_5x_5_params = {
		let rounds = get_rounds_poseidon_bls381_x5_5::<arkworks_gadgets::prelude::ark_bls12_381::Fr>();
		let mds = get_mds_poseidon_bls381_x5_5::<arkworks_gadgets::prelude::ark_bls12_381::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	let bn254_3x_5_params = {
		let rounds = get_rounds_poseidon_bn254_x3_5::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		let mds = get_mds_poseidon_bn254_x3_5::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	let bn254_5x_5_params = {
		let rounds = get_rounds_poseidon_bn254_x5_5::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		let mds = get_mds_poseidon_bn254_x5_5::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	let verifier_params = {
		use std::fs;
		// let pk_bytes = fs::read("../../fixtures/proving_key.bin").unwrap();
		let vk_bytes = fs::read("./fixtures/verifying_key.bin").unwrap();

		vk_bytes
	};

	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.into();
	// endow all authorities and nominators.
	initial_authorities
		.iter()
		.map(|x| &x.0)
		.chain(initial_nominators.iter())
		.for_each(|x| {
			if !endowed_accounts.contains(&x) {
				endowed_accounts.push(x.clone())
			}
		});

	let mut unique = vec![];
	unique.append(&mut endowed_accounts);
	unique.append(&mut initial_nominators.clone());
	unique.append(
		&mut initial_authorities
			.iter()
			.map(|x| &x.0)
			.map(|x| x.clone())
			.collect::<Vec<_>>(),
	);
	unique = unique
		.into_iter()
		.unique()
		.into_iter()
		.map(|x| x.clone())
		.collect::<Vec<_>>();

	// stakers: all validators and nominators.
	let mut rng = rand::thread_rng();

	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	const STASH: Balance = ENDOWMENT / 1000;

	GenesisConfig {
		system: darkwebb_runtime::SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: darkwebb_runtime::BalancesConfig {
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
						darkwebb_session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
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
		sudo: SudoConfig { key: root_key },
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(darkwebb_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		im_online: ImOnlineConfig { keys: vec![] },
		authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		grandpa: GrandpaConfig { authorities: vec![] },
		treasury: Default::default(),
		bls381_poseidon_3x_5_hasher: BLS381Poseidon3x5HasherConfig {
			parameters: Some(bls381_3x_5_params.to_bytes()),
			phantom: Default::default(),
		},
		bls381_poseidon_5x_5_hasher: BLS381Poseidon5x5HasherConfig {
			parameters: Some(bls381_5x_5_params.to_bytes()),
			phantom: Default::default(),
		},
		bn254_poseidon_3x_5_hasher: BN254Poseidon3x5HasherConfig {
			parameters: Some(bn254_3x_5_params.to_bytes()),
			phantom: Default::default(),
		},
		bn254_poseidon_5x_5_hasher: BN254Poseidon5x5HasherConfig {
			parameters: Some(bn254_5x_5_params.to_bytes()),
			phantom: Default::default(),
		},
		bn254_circom_poseidon_3x_5_hasher: BN254CircomPoseidon3x5HasherConfig {
			parameters: Some(circom_params.to_bytes()),
			phantom: Default::default(),
		},
		verifier: VerifierConfig {
			parameters: Some(verifier_params),
			phantom: Default::default(),
		},
		merkle_tree: MerkleTreeConfig {
			phantom: Default::default(),
			default_hashes: None,
		},
	}
}
