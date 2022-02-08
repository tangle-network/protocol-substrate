use ark_ff::{BigInteger, PrimeField};
use ark_std::{rand::thread_rng, vec::Vec};
use arkworks_circuits::{
	setup::{common::{
		prove_unchecked,
	}, vanchor::Utxo},
	setup::vanchor::{VAnchorProverBn2542x2},
};
use arkworks_utils::{
	utils::common::{
		setup_params_x5_2, setup_params_x5_3, setup_params_x5_4, setup_params_x5_5, Curve,
	},
};
use webb_primitives::{
	ElementTrait,
};

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;
type Bn254 = ark_bn254::Bn254;

const TREE_DEPTH: usize = 30;
const M: usize = 2;
const N: usize = 2;

pub fn setup_utxos(
	// Transaction inputs
	chain_ids: [u128; N],
	amounts: [u128; N],
	indices: Option<[u64; N]>
) -> [Utxo<Bn254Fr>; N] {
	let rng = &mut thread_rng();

	let params2 = setup_params_x5_2::<Bn254Fr>(Curve::Bn254);
	let params3 = setup_params_x5_3::<Bn254Fr>(Curve::Bn254);
	let params4 = setup_params_x5_4::<Bn254Fr>(Curve::Bn254);
	let params5 = setup_params_x5_5::<Bn254Fr>(Curve::Bn254);

	let prover = VAnchorProverBn2542x2::new(params2, params3, params4, params5);
	// Input Utxos
	let chain_id1 = Bn254Fr::from(chain_ids[0]);
	let chain_id2 = Bn254Fr::from(chain_ids[1]);
	let amount1 = Bn254Fr::from(amounts[0]);
	let amount2 = Bn254Fr::from(amounts[1]);
	let indices: [Option<Bn254Fr>; N] = if indices.is_some() {
		let ind_unw = indices.unwrap();
		ind_unw.map(|x| Some(Bn254Fr::from(x)))
	} else {
		[None; N]
	};
	let utxo1 = prover
		.new_utxo(chain_id1, amount1, indices[0], None, None, rng)
		.unwrap();
	let utxo2 = prover
		.new_utxo(chain_id2, amount2, indices[1], None, None, rng)
		.unwrap();
	let in_utxos = [utxo1, utxo2];

	in_utxos
}

pub fn setup_zk_circuit(
	// Metadata inputs
	public_amount: i128,
	ext_data_hash: Vec<u8>,
	in_utxos: [Utxo<Bn254Fr>; N],
	out_utxos: [Utxo<Bn254Fr>; N],
	custom_roots: Option<[Vec<u8>; M]>,
	pk_bytes: &Vec<u8>
) -> (Vec<u8>, Vec<Bn254Fr>) {
	let rng = &mut thread_rng();

	let params2 = setup_params_x5_2::<Bn254Fr>(Curve::Bn254);
	let params3 = setup_params_x5_3::<Bn254Fr>(Curve::Bn254);
	let params4 = setup_params_x5_4::<Bn254Fr>(Curve::Bn254);
	let params5 = setup_params_x5_5::<Bn254Fr>(Curve::Bn254);

	let prover = VAnchorProverBn2542x2::new(params2, params3, params4, params5);

	// Make a proof now
	let public_amount = Bn254Fr::from(public_amount);

	let leaf0 = in_utxos[0].commitment;
	let leaf1 = in_utxos[1].commitment;

	let leaves = vec![leaf0, leaf1];

	let in_leaves = [leaves.clone(), leaves.clone()];
	let in_indices = [0, 1];

	// This allows us to pass zero roots for initial transaction
	let in_root_set = if custom_roots.is_some() {
		let custom_roots_bytes = custom_roots.unwrap();
		custom_roots_bytes.map(|x| Bn254Fr::from_le_bytes_mod_order(&x))
	} else {
		let (_, root) = prover.setup_tree(&leaves, 0).unwrap();
		[root; M]
	};

	let ext_data_hash_f = Bn254Fr::from_le_bytes_mod_order(&ext_data_hash);

	let (circuit, .., pub_ins) = prover
		.setup_circuit_with_utxos(
			public_amount,
			ext_data_hash_f,
			in_root_set,
			in_indices,
			in_leaves,
			in_utxos,
			out_utxos,
		)
		.unwrap();

	let proof = prove_unchecked::<Bn254, _, _>(circuit, pk_bytes, rng).unwrap();

	(proof, pub_ins)
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

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[12..].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
