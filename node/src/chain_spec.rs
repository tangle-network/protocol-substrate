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
use node_template_runtime::{
	AccountId, AuraConfig, BLS381Poseidon3x5HasherConfig, BLS381Poseidon5x5HasherConfig,
	BN254CircomPoseidon3x5HasherConfig, BN254Poseidon3x5HasherConfig, BN254Poseidon5x5HasherConfig, BalancesConfig,
	GenesisConfig, GrandpaConfig, Signature, SudoConfig, SystemConfig, VerifierConfig, WASM_BINARY,
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate
/// ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				true,
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
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
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
				true,
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
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
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
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: root_key,
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
