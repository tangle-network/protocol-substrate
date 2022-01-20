use arkworks_gadgets::prelude::ark_bn254::Bn254;
use arkworks_utils::{
	poseidon::PoseidonParameters,
	utils::common::{setup_params_x5_3, setup_params_x5_5, Curve},
};
use common::{AccountId, AuraId, Signature};
use darkwebb_runtime::{
	wasm_binary_unwrap, AssetRegistryConfig, AuraConfig, BalancesConfig, CouncilConfig, GenesisConfig,
	HasherBls381Config, HasherBn254Config, MerkleTreeBls381Config, MerkleTreeBn254Config, MixerBn254Config,
	ParachainStakingConfig, SudoConfig, SystemConfig, VerifierBls381Config, VerifierBn254Config, KUNITS, UNITS,
};
use webb_primitives::Balance;

use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use pallet_parachain_staking::{InflationInfo, Range};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill, Percent,
};

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
		Extensions {
			relay_chain: "rococo".into(),
			para_id: id.into(),
		},
	))
}

pub const ENDOWMENT: u128 = UNITS * 4096_000;

pub fn darkwebb_test_genesis_inflation_config(endowed_accounts: Vec<AccountId>) -> InflationInfo<Balance> {
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
	let curve_bls381 = Curve::Bls381;
	log::info!("Bn254 x5 w3 params");
	let bn254_x5_3_params = setup_params_x5_3::<ark_bn254::Fr>(curve_bn254);

	log::info!("BLS381 x5 w3 params");
	let bls381_x5_3_params = setup_params_x5_3::<ark_bls12_381::Fr>(curve_bls381);

	log::info!("Verifier params");
	let verifier_params = {
		use std::fs;
		// let pk_bytes = fs::read("../../fixtures/proving_key.bin").unwrap();
		let vk_bytes = include_bytes!("../../protocol-substrate-fixtures/mixer/bn254/x5/verifying_key.bin");

		vk_bytes.to_vec()
	};

	log::info!("Genesis Config");
	GenesisConfig {
		system: darkwebb_runtime::SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
		},
		asset_registry: AssetRegistryConfig {
			asset_names: vec![],
			native_asset_name: b"WEBB".to_vec(),
			native_existential_deposit: darkwebb_runtime::constants::currency::EXISTENTIAL_DEPOSIT,
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
			key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
		},
		hasher_bn_254: HasherBn254Config {
			parameters: Some(bn254_x5_3_params.to_bytes()),
			phantom: Default::default(),
		},
		hasher_bls_381: HasherBls381Config {
			parameters: Some(bls381_x5_3_params.to_bytes()),
			phantom: Default::default(),
		},
		verifier_bn_254: VerifierBn254Config {
			parameters: Some(verifier_params),
			phantom: Default::default(),
		},
		verifier_bls_381: VerifierBls381Config {
			parameters: None,
			phantom: Default::default(),
		},
		merkle_tree_bn_254: MerkleTreeBn254Config {
			phantom: Default::default(),
			default_hashes: None,
		},
		merkle_tree_bls_381: MerkleTreeBls381Config {
			phantom: Default::default(),
			default_hashes: None,
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
			inflation_config: darkwebb_test_genesis_inflation_config(endowed_accounts),
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
