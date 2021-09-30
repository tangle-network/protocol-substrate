use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	prelude::ark_bn254::Bn254,
	setup::{common::Curve, mixer::setup_groth16_random_circuit_circomx5},
	utils::{
		get_mds_poseidon_bls381_x3_5, get_mds_poseidon_bls381_x5_5, get_mds_poseidon_bn254_x3_5,
		get_mds_poseidon_bn254_x5_5, get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_bls381_x3_5,
		get_rounds_poseidon_bls381_x5_5, get_rounds_poseidon_bn254_x3_5, get_rounds_poseidon_bn254_x5_5,
		get_rounds_poseidon_circom_bn254_x5_3,
	},
};
use common::{AccountId, AuraId, Signature};
use darkwebb_runtime::{
	wasm_binary_unwrap, AuraConfig, BLS381Poseidon3x5HasherConfig, BLS381Poseidon5x5HasherConfig,
	BN254CircomPoseidon3x5HasherConfig, BN254Poseidon3x5HasherConfig, BN254Poseidon5x5HasherConfig, BalancesConfig,
	GenesisConfig, SudoConfig, SystemConfig, VerifierConfig,
};

use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<darkwebb_runtime::GenesisConfig, Extensions>;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type DarkwebbChainSpec = sc_service::GenericChainSpec<darkwebb_runtime::GenesisConfig, Extensions>;

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
pub fn darkwebb_session_keys(keys: AuraId) -> darkwebb_runtime::SessionKeys {
	darkwebb_runtime::SessionKeys { aura: keys }
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
		Extensions {
			relay_chain: "westend".into(),
			para_id: id.into(),
		},
	)
}

pub fn darkwebb_development_config(id: ParaId) -> Result<ChainSpec, String> {
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
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_collator_keys_from_seed("Bob"),
					),
				],
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
		// Properties
		None,
		Extensions {
			relay_chain: "kusama".into(),
			para_id: id.into(),
		},
	))
}

pub fn darkwebb_local_testnet_config(id: ParaId) -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				// initial collators.
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						get_collator_keys_from_seed("Alice"),
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_collator_keys_from_seed("Bob"),
					),
				],
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
		// Properties
		None,
		// Extensions
		Extensions {
			relay_chain: "rococo".into(),
			para_id: id.into(),
		},
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> GenesisConfig {
	use ark_serialize::CanonicalSerialize;
	use ark_std::test_rng;
	log::info!("Circom params");
	let circom_params = {
		let rounds = get_rounds_poseidon_circom_bn254_x5_3::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		let mds = get_mds_poseidon_circom_bn254_x5_3::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	log::info!("BLS381 3x 5 params");
	let bls381_3x_5_params = {
		let rounds = get_rounds_poseidon_bls381_x3_5::<arkworks_gadgets::prelude::ark_bls12_381::Fr>();
		let mds = get_mds_poseidon_bls381_x3_5::<arkworks_gadgets::prelude::ark_bls12_381::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	log::info!("BLS381 5x 5 params");
	let bls381_5x_5_params = {
		let rounds = get_rounds_poseidon_bls381_x5_5::<arkworks_gadgets::prelude::ark_bls12_381::Fr>();
		let mds = get_mds_poseidon_bls381_x5_5::<arkworks_gadgets::prelude::ark_bls12_381::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	log::info!("BN254 3x 5 params");
	let bn254_3x_5_params = {
		let rounds = get_rounds_poseidon_bn254_x3_5::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		let mds = get_mds_poseidon_bn254_x3_5::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	log::info!("BN254 5x 5 params");
	let bn254_5x_5_params = {
		let rounds = get_rounds_poseidon_bn254_x5_5::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		let mds = get_mds_poseidon_bn254_x5_5::<arkworks_gadgets::prelude::ark_bn254::Fr>();
		PoseidonParameters::new(rounds, mds)
	};

	log::info!("Verifier params");
	let verifier_params = {
		use std::fs;
		// let pk_bytes = fs::read("../../fixtures/proving_key.bin").unwrap();
		let vk_bytes = fs::read("./fixtures/verifying_key.bin").unwrap();

		vk_bytes
	};

	log::info!("Genesis Config");
	GenesisConfig {
		system: darkwebb_runtime::SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: darkwebb_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, darkwebb_runtime::constants::currency::EXISTENTIAL_DEPOSIT * 4096))
				.collect(),
		},
		parachain_info: darkwebb_runtime::ParachainInfoConfig { parachain_id: id },
		collator_selection: darkwebb_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: darkwebb_runtime::constants::currency::EXISTENTIAL_DEPOSIT * 16,
			..Default::default()
		},
		session: darkwebb_runtime::SessionConfig {
			keys: invulnerables
				.iter()
				.cloned()
				.map(|(acc, aura)| {
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
	}
}

fn shell_testnet_genesis(parachain_id: ParaId) -> shell_runtime::GenesisConfig {
	shell_runtime::GenesisConfig {
		system: shell_runtime::SystemConfig {
			code: shell_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		parachain_info: shell_runtime::ParachainInfoConfig { parachain_id },
		parachain_system: Default::default(),
	}
}
