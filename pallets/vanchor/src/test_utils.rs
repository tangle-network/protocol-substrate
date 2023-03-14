use crate::tests::{
	BIGGER_DEFAULT_BALANCE, BIGGER_TRANSACTOR_ACCOUNT_ID, BIG_DEFAULT_BALANCE,
	BIG_TRANSACTOR_ACCOUNT_ID, DEFAULT_BALANCE, RELAYER_ACCOUNT_ID, SEED, TRANSACTOR_ACCOUNT_ID,
};
use ark_bn254::Fr;
use ark_circom::WitnessCalculator;
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{
	create_proof_with_reduction_and_matrices, create_random_proof as prove,
	generate_random_parameters, prepare_verifying_key, verify_proof as ark_verify_proof,
	Proof as ArkProof, ProvingKey, VerifyingKey,
};
use ark_relations::r1cs::ConstraintMatrices;
use ark_serialize::CanonicalSerialize;
use ark_std::{rand::thread_rng, vec::Vec};
use arkworks_native_gadgets::poseidon::Poseidon;
use arkworks_setups::{
	common::{setup_params, setup_tree_and_create_path},
	r1cs::vanchor::VAnchorR1CSProver,
	utxo::Utxo,
	Curve, VAnchorProver,
};
use circom_proving::circom_from_folder;
use std::{collections::BTreeMap, convert::TryInto, sync::Mutex};
use webb_primitives::ElementTrait;

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;
type Bn254 = ark_bn254::Bn254;

pub const TREE_DEPTH: usize = 30;
pub const ANCHOR_CT: usize = 2;
pub const NUM_UTXOS: usize = 2;
pub const DEFAULT_LEAF: [u8; 32] = [
	47, 229, 76, 96, 211, 172, 171, 243, 52, 58, 53, 182, 235, 161, 93, 180, 130, 27, 52, 15, 118,
	231, 65, 226, 36, 150, 133, 237, 72, 153, 175, 108,
];

#[allow(non_camel_case_types)]
type VAnchorProver_Bn254_30_2_2_2 =
	VAnchorR1CSProver<Bn254, TREE_DEPTH, ANCHOR_CT, NUM_UTXOS, NUM_UTXOS>;

use crate::{mock::*, zerokit_utils::*, Instance2};
use ark_bn254::{Fq, Fq2, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_circom::{read_zkey, CircomConfig, CircomReduction};
use ark_ff::{BigInteger256, ToBytes};
use ark_relations::r1cs::SynthesisError;
use ark_std::UniformRand;
use arkworks_native_gadgets::merkle_tree::{Path, SparseMerkleTree};
use cfg_if::cfg_if;
use frame_benchmarking::account;
use frame_support::{assert_ok, traits::OnInitialize};
use num_bigint::{BigUint, Sign};
use once_cell::sync::OnceCell;
use pallet_linkable_tree::LinkableTreeConfigration;
use serde_json::Value;
use sp_core::hashing::keccak_256;
use std::{
	convert::TryFrom,
	fs::{self, File},
	io::{Cursor, Error, ErrorKind},
	result::Result,
	str::FromStr,
};
use thiserror::Error;
use wasmer::{Module, Store};
use webb_primitives::{
	linkable_tree::LinkableTreeInspector,
	merkle_tree::TreeInspector,
	types::vanchor::{ExtData, ProofData},
	utils::compute_chain_id_type,
	verifying::CircomError,
	AccountId,
};

pub fn setup_environment_with_circom(
) -> ((ProvingKey<Bn254>, ConstraintMatrices<Fr>), &'static Mutex<WitnessCalculator>) {
	let curve = Curve::Bn254;
	let params3 = setup_params::<ark_bn254::Fr>(curve, 5, 3);
	// 1. Setup The Hasher Pallet.
	println!("Setting up the hasher pallet");
	assert_ok!(Hasher2::force_set_parameters(
		RuntimeOrigin::root(),
		params3.to_bytes().try_into().unwrap()
	));
	// 2. Initialize MerkleTree pallet.
	println!("Initializing the merkle tree pallet");
	<MerkleTree2 as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey

	// Load the WASM and R1CS for witness and proof generation
	// Get path to solidity fixtures
	println!("Setting up ZKey");
	let path_2_2 = "../../solidity-fixtures/solidity-fixtures/vanchor_2/2/circuit_final.zkey";
	let mut file_2_2 = File::open(path_2_2).unwrap();
	let params_2_2 = read_zkey(&mut file_2_2).unwrap();

	println!("Setting up the verifier pallet");
	let mut vk_2_2_bytes = Vec::new();
	params_2_2.0.vk.serialize(&mut vk_2_2_bytes).unwrap();

	assert_ok!(VAnchorVerifier2::force_set_parameters(
		RuntimeOrigin::root(),
		(2, 2),
		vk_2_2_bytes.try_into().unwrap(),
	));

	let wasm_2_2_path =
		"../../solidity-fixtures/solidity-fixtures//vanchor_2/2/poseidon_vanchor_2_2.wasm";

	let wc_2_2 = circom_from_folder(wasm_2_2_path);

	let transactor = account::<AccountId>("", TRANSACTOR_ACCOUNT_ID, SEED);
	let relayer = account::<AccountId>("", RELAYER_ACCOUNT_ID, SEED);
	let big_transactor = account::<AccountId>("", BIG_TRANSACTOR_ACCOUNT_ID, SEED);
	let bigger_transactor = account::<AccountId>("", BIGGER_TRANSACTOR_ACCOUNT_ID, SEED);

	// Set balances
	assert_ok!(Balances::set_balance(RuntimeOrigin::root(), transactor, DEFAULT_BALANCE, 0));
	assert_ok!(Balances::set_balance(RuntimeOrigin::root(), relayer, DEFAULT_BALANCE, 0));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		big_transactor,
		BIG_DEFAULT_BALANCE,
		0
	));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		bigger_transactor,
		BIGGER_DEFAULT_BALANCE,
		0
	));

	// set configurable storage
	assert_ok!(VAnchor2::set_max_deposit_amount(RuntimeOrigin::root(), 10, 1));
	assert_ok!(VAnchor2::set_min_withdraw_amount(RuntimeOrigin::root(), 3, 2));

	// finally return the provingkey bytes
	(params_2_2, wc_2_2)
}

pub fn setup_utxos(
	// Transaction inputs
	chain_ids: [u64; NUM_UTXOS],
	amounts: [u128; NUM_UTXOS],
	indices: Option<[u64; NUM_UTXOS]>,
) -> [Utxo<Bn254Fr>; NUM_UTXOS] {
	let curve = Curve::Bn254;
	let rng = &mut thread_rng();
	// Input Utxos
	let indices: [Option<u64>; NUM_UTXOS] = if indices.is_some() {
		let ind_unw = indices.unwrap();
		ind_unw.map(Some)
	} else {
		[None; NUM_UTXOS]
	};
	let utxo1 = VAnchorProver_Bn254_30_2_2_2::create_random_utxo(
		curve,
		chain_ids[0],
		amounts[0],
		indices[0],
		rng,
	)
	.unwrap();
	let utxo2 = VAnchorProver_Bn254_30_2_2_2::create_random_utxo(
		curve,
		chain_ids[1],
		amounts[1],
		indices[1],
		rng,
	)
	.unwrap();

	[utxo1, utxo2]
}

pub fn setup_zk_circuit(
	// Metadata inputs
	public_amount: i128,
	chain_id: u64,
	ext_data_hash: Vec<u8>,
	in_utxos: [Utxo<Bn254Fr>; NUM_UTXOS],
	out_utxos: [Utxo<Bn254Fr>; NUM_UTXOS],
	pk_bytes: Vec<u8>,
	neighbor_roots: [Element; ANCHOR_CT - 1],
	custom_root: Element,
) -> (Vec<u8>, Vec<Bn254Fr>) {
	let curve = Curve::Bn254;
	let rng = &mut thread_rng();

	let leaf0 = in_utxos[0].commitment.into_repr().to_bytes_be();
	let leaf1 = in_utxos[1].commitment.into_repr().to_bytes_be();

	let leaves: Vec<Vec<u8>> = vec![leaf0, leaf1];
	let leaves_f: Vec<Bn254Fr> =
		leaves.iter().map(|x| Bn254Fr::from_be_bytes_mod_order(x)).collect();

	let mut in_leaves: BTreeMap<u64, Vec<Vec<u8>>> = BTreeMap::new();
	in_leaves.insert(chain_id, leaves);
	let in_indices = [0, 1];

	let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
	let poseidon3 = Poseidon::new(params3);
	let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
		&poseidon3,
		&leaves_f,
		0,
		&DEFAULT_LEAF,
	)
	.unwrap();

	let roots_f: [Bn254Fr; ANCHOR_CT] = vec![if custom_root != Element::from_bytes(&[0u8; 32]) {
		Bn254Fr::from_be_bytes_mod_order(custom_root.to_bytes())
	} else {
		tree.root()
	}]
	.iter()
	.chain(
		neighbor_roots
			.iter()
			.map(|r| Bn254Fr::from_be_bytes_mod_order(r.to_bytes()))
			.collect::<Vec<Bn254Fr>>()
			.iter(),
	)
	.cloned()
	.collect::<Vec<Bn254Fr>>()
	.try_into()
	.unwrap();
	let in_root_set = roots_f.map(|x| x.into_repr().to_bytes_be());

	let vanchor_proof = VAnchorProver_Bn254_30_2_2_2::create_proof(
		curve,
		chain_id,
		public_amount,
		ext_data_hash,
		in_root_set,
		in_indices,
		in_leaves,
		in_utxos,
		out_utxos,
		pk_bytes,
		DEFAULT_LEAF,
		rng,
	)
	.unwrap();

	let pub_ins = vanchor_proof
		.public_inputs_raw
		.iter()
		.map(|x| Bn254Fr::from_be_bytes_mod_order(x))
		.collect();

	(vanchor_proof.proof, pub_ins)
}

pub fn deconstruct_public_inputs(
	public_inputs: &Vec<Bn254Fr>,
) -> (
	Bn254Fr,      // Chain Id
	Bn254Fr,      // Public amount
	Vec<Bn254Fr>, // Roots
	Vec<Bn254Fr>, // Input tx Nullifiers
	Vec<Bn254Fr>, // Output tx commitments
	Bn254Fr,      // External data hash
) {
	let public_amount = public_inputs[0];
	let ext_data_hash = public_inputs[1];
	let nullifiers = public_inputs[2..4].to_vec();
	let commitments = public_inputs[4..6].to_vec();
	let chain_id = public_inputs[6];
	let root_set = public_inputs[7..9].to_vec();
	(chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash)
}

pub fn deconstruct_public_inputs_el(
	public_inputs_f: &Vec<Bn254Fr>,
) -> (
	Element,      // Chain Id
	Element,      // Public amount
	Vec<Element>, // Roots
	Vec<Element>, // Input tx Nullifiers
	Vec<Element>, // Output tx commitments
	Element,      // External amount
) {
	let (chain_id, public_amount, roots, nullifiers, commitments, ext_data_hash) =
		deconstruct_public_inputs(public_inputs_f);
	let chain_id_el = Element::from_bytes(&chain_id.into_repr().to_bytes_be());
	let public_amount_el = Element::from_bytes(&public_amount.into_repr().to_bytes_be());
	let root_set_el = roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();
	let nullifiers_el = nullifiers
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();
	let commitments_el = commitments
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();
	let ext_data_hash_el = Element::from_bytes(&ext_data_hash.into_repr().to_bytes_be());
	(chain_id_el, public_amount_el, root_set_el, nullifiers_el, commitments_el, ext_data_hash_el)
}
