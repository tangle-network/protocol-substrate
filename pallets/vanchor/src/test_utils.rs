use std::collections::BTreeMap;

use ark_ff::{BigInteger, PrimeField};
use ark_std::{rand::thread_rng, vec::Vec};
use arkworks_native_gadgets::poseidon::Poseidon;
use arkworks_setups::{
	common::{setup_params, setup_tree_and_create_path},
	r1cs::vanchor::VAnchorR1CSProver,
	utxo::Utxo,
	Curve, VAnchorProver,
};
use webb_primitives::ElementTrait;

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;
type Bn254 = ark_bn254::Bn254;

const TREE_DEPTH: usize = 30;
const ANCHOR_CT: usize = 2;
const NUM_UTXOS: usize = 2;
pub const DEFAULT_LEAF: [u8; 32] = [
	108, 175, 153, 072, 237, 133, 150, 036, 226, 065, 231, 118, 015, 052, 027, 130, 180, 093, 161,
	235, 182, 053, 058, 052, 243, 171, 172, 211, 096, 076, 229, 047,
];

type VAnchorProver_Bn254_30_2_2_2 =
	VAnchorR1CSProver<Bn254, TREE_DEPTH, ANCHOR_CT, NUM_UTXOS, NUM_UTXOS>;

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
		ind_unw.map(|x| Some(x))
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
	let in_utxos = [utxo1, utxo2];

	in_utxos
}

pub fn setup_zk_circuit(
	// Metadata inputs
	public_amount: i128,
	chain_id: u64,
	ext_data_hash: Vec<u8>,
	in_utxos: [Utxo<Bn254Fr>; NUM_UTXOS],
	out_utxos: [Utxo<Bn254Fr>; NUM_UTXOS],
	custom_roots: Option<[Vec<u8>; ANCHOR_CT]>,
	pk_bytes: Vec<u8>,
) -> (Vec<u8>, Vec<Bn254Fr>) {
	let curve = Curve::Bn254;
	let rng = &mut thread_rng();

	let leaf0 = in_utxos[0].commitment.into_repr().to_bytes_le();
	let leaf1 = in_utxos[1].commitment.into_repr().to_bytes_le();

	let leaves: Vec<Vec<u8>> = vec![leaf0, leaf1];
	let leaves_f: Vec<Bn254Fr> =
		leaves.iter().map(|x| Bn254Fr::from_le_bytes_mod_order(&x)).collect();

	let mut in_leaves: BTreeMap<u64, Vec<Vec<u8>>> = BTreeMap::new();
	in_leaves.insert(chain_id, leaves);
	let in_indices = [0, 1];

	// This allows us to pass zero roots for initial transaction
	let in_root_set = if custom_roots.is_some() {
		custom_roots.unwrap()
	} else {
		let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
		let poseidon3 = Poseidon::new(params3);
		let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
			&poseidon3,
			&leaves_f,
			0,
			&DEFAULT_LEAF,
		)
		.unwrap();
		[(); ANCHOR_CT].map(|_| tree.root().into_repr().to_bytes_le())
	};

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
		pk_bytes.clone(),
		DEFAULT_LEAF,
		rng,
	)
	.unwrap();

	let pub_ins = vanchor_proof
		.public_inputs_raw
		.iter()
		.map(|x| Bn254Fr::from_le_bytes_mod_order(x))
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
	let chain_id_el = Element::from_bytes(&chain_id.into_repr().to_bytes_le());
	let public_amount_el = Element::from_bytes(&public_amount.into_repr().to_bytes_le());
	let root_set_el = roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let nullifiers_el = nullifiers
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let commitments_el = commitments
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let ext_data_hash_el = Element::from_bytes(&ext_data_hash.into_repr().to_bytes_le());
	(chain_id_el, public_amount_el, root_set_el, nullifiers_el, commitments_el, ext_data_hash_el)
}
