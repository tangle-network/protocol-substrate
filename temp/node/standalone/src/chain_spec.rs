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
use common::{AccountId, AuraId, AuthorityDiscoveryId, BabeId, ImOnlineId, Signature};
use darkwebb_runtime::{
	AuthorityDiscoveryConfig, BLS381Poseidon3x5HasherConfig, BLS381Poseidon5x5HasherConfig,
	BN254CircomPoseidon3x5HasherConfig, BN254Poseidon3x5HasherConfig, BN254Poseidon5x5HasherConfig, BabeConfig,
	BalancesConfig, CouncilConfig, DemocracyConfig, ElectionsConfig, GenesisConfig, GrandpaConfig, ImOnlineConfig,
	IndicesConfig, StakingConfig, SudoConfig, SystemConfig, VerifierConfig, WASM_BINARY,
};
use hex_literal::hex;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<darkwebb_runtime::GenesisConfig, Extensions>;

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
) -> darkwebb_runtime::SessionKeysSessionKeys {
	darkwebb_runtime::SessionKeys {
		grandpa,
		babe,
		im_online,
		authority_discovery,
	}
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
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
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
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
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
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	endowed_accounts: Vec<AccountId>,
	root_key: AccountId,
) -> GenesisConfig {
	use ark_serialize::CanonicalSerialize;
	use ark_std::test_rng;

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
		let mut rng = test_rng();
		pub const LEN: usize = 30;
		let curve = Curve::Bn254;
		let (pk, vk) = setup_groth16_random_circuit_circomx5::<_, Bn254, LEN>(&mut rng, curve);

		let mut serialized = vec![0; vk.serialized_size()];
		vk.serialize(&mut serialized[..]).unwrap();

		serialized
	};

	GenesisConfig {
		system: darkwebb_runtime::SystemConfig {
			code: darkwebb_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: darkwebb_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, darkwebb_runtime::constants::currency::EXISTENTIAL_DEPOSIT * 4096))
				.collect(),
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
			stakers,
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
			epoch_config: Some(node_runtime::BABE_GENESIS_EPOCH_CONFIG),
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
	}
}
