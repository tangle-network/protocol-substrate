use arkworks_circuits::setup::mixer::setup_groth16_random_circuit_x5;
use arkworks_gadgets::prelude::ark_bn254::Bn254;
use arkworks_utils::{
	poseidon::PoseidonParameters,
	utils::common::{setup_params_x3_5, setup_params_x5_3, setup_params_x5_5, Curve},
};
use common::{AccountId, BabeId, Balance, Signature};

use darkwebb_runtime::{
	constants::currency::*, wasm_binary_unwrap, AnchorVerifierConfig, AuthorityDiscoveryConfig,
	BLS381Poseidon3x5HasherConfig, BLS381Poseidon5x5HasherConfig, BN254CircomPoseidon3x5HasherConfig,
	BN254Poseidon3x5HasherConfig, BN254Poseidon5x5HasherConfig, BabeConfig, Block, CouncilConfig, DemocracyConfig,
	ElectionsConfig, GenesisConfig, GrandpaConfig, ImOnlineConfig, IndicesConfig, MerkleTreeConfig,
	MixerVerifierConfig, SessionConfig, StakerStatus, StakingConfig, SudoConfig,
};
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
	candidates: Vec<(AccountId, AuraId, Balance)>,
	nominations: Vec<(AccountId, AccountId, Balance)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> GenesisConfig {
	let curve_bn254 = Curve::Bn254;
	let curve_bls381 = Curve::Bls381;
	log::info!("Bn254 params");
	let bn254_params = setup_params_x5_3::<ark_bn254::Fr>(curve_bn254);

	log::info!("Bls381 params");
	let bls381_params = setup_params_x5_3::<ark_bls381::Fr>(curve_bls381);

	log::info!("Verifier params");
	let verifier_params = {
		use std::fs;
		// let pk_bytes = fs::read("../../fixtures/proving_key.bin").unwrap();
		let vk_bytes = include_bytes!("../../fixtures/verifying_key.bin");

		vk_bytes.to_vec()
	};

	log::info!("Genesis Config");
	GenesisConfig {
		system: darkwebb_runtime::SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: darkwebb_runtime::BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect(),
		},
		parachain_info: darkwebb_runtime::ParachainInfoConfig { parachain_id: id },
		session: darkwebb_runtime::SessionConfig {
			keys: candidates
				.iter()
				.cloned()
				.map(|(acc, aura, _)| {
					(
						acc.clone(),                 // account id
						acc.clone(),                 // validator id
						darkwebb_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: get_account_id_from_seed::<sr25519::Public>("Alice"),
		},
		poseidon_hasher_bls381: PoseidonBls381HasherConfig {
			parameters: Some(bls381_params.to_bytes()),
			phantom: Default::default(),
		},
		poseidon_hasher_bn254: PoseidonHasherBn254Config {
			parameters: Some(bn254_params.to_bytes()),
			phantom: Default::default(),
		},
		verifier_bn254: VerifierConfig {
			parameters: Some(verifier_params.clone()),
			phantom: Default::default(),
		},
		anchor_verifier: AnchorVerifierConfig {
			parameters: Some(verifier_params),
			phantom: Default::default(),
		},
		merkle_tree: MerkleTreeConfig {
			phantom: Default::default(),
			default_hashes: None,
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
			inflation_config: darkwebb_test_genesis_inflation_config(endowed_accounts),
		},
	}
}
