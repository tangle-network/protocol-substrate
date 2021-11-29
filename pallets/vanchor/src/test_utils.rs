use crate::mock::*;
use ark_crypto_primitives::snark::SNARK;
use ark_ff::{BigInteger, FromBytes, PrimeField, UniformRand};
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{
	rand::{thread_rng, CryptoRng, Rng, RngCore},
	rc::Rc,
	vec::Vec,
};
use arkworks_gadgets::{
	arbitrary::vanchor_data::VAnchorArbitraryData,
	circuit::vanchor::VAnchorCircuit as VACircuit,
	keypair::vanchor::Keypair,
	leaf::vanchor::{Private as LeafPrivateInput, Public as LeafPublicInput, VAnchorLeaf as Leaf},
	merkle_tree::{Path, SparseMerkleTree},
	poseidon::PoseidonParameters,
	set::membership::{Private as SetPrivateInputs, SetMembership},
	setup::common::{
		setup_params_x5_2, setup_params_x5_3, setup_params_x5_4, setup_params_x5_5, Curve, LeafCRH, LeafCRHGadget,
		PoseidonCRH_x5_2, PoseidonCRH_x5_2Gadget, PoseidonCRH_x5_3, PoseidonCRH_x5_3Gadget, PoseidonCRH_x5_4,
		PoseidonCRH_x5_4Gadget, PoseidonCRH_x5_5, PoseidonCRH_x5_5Gadget, TreeConfig_x5, Tree_x5,
	},
};
use darkwebb_primitives::ElementTrait;

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;
type Bn254 = ark_bn254::Bn254;
type Bls12_381Fr = ark_bls12_381::Fr;

type ProofBytes = Vec<u8>;
type RootsElement = Vec<Element>;
type NullifierHashElement = Element;
type LeafElement = Element;

const TREE_DEPTH: usize = 30;
const M: usize = 2;
const INS: usize = 2;
const OUTS: usize = 2;

pub fn get_hash_params<F: PrimeField>(
	curve: Curve,
) -> (
	PoseidonParameters<F>,
	PoseidonParameters<F>,
	PoseidonParameters<F>,
	PoseidonParameters<F>,
) {
	(
		setup_params_x5_2::<F>(curve),
		setup_params_x5_3::<F>(curve),
		setup_params_x5_4::<F>(curve),
		setup_params_x5_5::<F>(curve),
	)
}

pub fn setup_circuit_with_inputs(
	chain_ids: Vec<Bn254Fr>,
	public_amount: Bn254Fr,
	in_amounts: Vec<Bn254Fr>,
	out_amounts: Vec<Bn254Fr>,
	ext_data: Bn254Fr,
) -> (
	VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		PoseidonCRH_x5_4<Bn254Fr>,
		PoseidonCRH_x5_4Gadget<Bn254Fr>,
		PoseidonCRH_x5_5<Bn254Fr>,
		PoseidonCRH_x5_5Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
	Vec<Bn254Fr>,
) {
	let rng = &mut thread_rng();
	let (params2, params3, params4, params5) = get_hash_params::<ark_bn254::Fr>(Curve::Bn254);

	// Arbitrary data
	let arbitrary_data = setup_arbitrary_data(ext_data);

	// Input leaves (txos)
	let in_keypairs = setup_keypairs(in_amounts.len());
	let (in_leaves, in_nullifiers, in_leaf_privates, in_leaf_publics) =
		setup_leaves(&chain_ids, &in_amounts, &in_keypairs);

	// Tree + set for proving input txos
	let out_pub_keys: Vec<Bn254Fr> = in_keypairs.iter().map(|x| x.public_key(&params2).unwrap()).collect();
	let (in_paths, in_indices, in_root_set, in_set_private_inputs) = setup_tree_and_set(&in_leaves);

	// Output leaves (txos)
	let out_keypairs = setup_keypairs(in_amounts.len());
	let (out_commitments, _out_nullifiers, out_leaf_privates, out_leaf_publics) =
		setup_leaves(&chain_ids, &out_amounts, &out_keypairs);

	setup_circuit(
		public_amount,
		arbitrary_data,
		in_keypairs,
		in_leaf_privates,
		in_leaf_publics[0].clone(),
		in_nullifiers,
		in_indices,
		in_paths,
		in_set_private_inputs,
		in_root_set,
		out_leaf_privates,
		out_leaf_publics,
		out_commitments,
		out_pub_keys,
	)
}

pub fn setup_circuit(
	public_amount: Bn254Fr,
	arbitrary_data: VAnchorArbitraryData<Bn254Fr>,
	// Input transactions
	in_keypairs: Vec<Keypair<Bn254Fr, PoseidonCRH_x5_2<Bn254Fr>>>,
	in_leaf_privates: Vec<LeafPrivateInput<Bn254Fr>>,
	in_leaf_public: LeafPublicInput<Bn254Fr>,
	in_nullifiers: Vec<Bn254Fr>,
	in_indicies: Vec<Bn254Fr>,
	// Data related to tree
	in_paths: Vec<Path<TreeConfig_x5<Bn254Fr>, TREE_DEPTH>>,
	in_set_private_inputs: Vec<SetPrivateInputs<Bn254Fr, M>>,
	in_root_set: [Bn254Fr; M],
	// Output transactions
	out_leaf_privates: Vec<LeafPrivateInput<Bn254Fr>>,
	out_leaf_publics: Vec<LeafPublicInput<Bn254Fr>>,
	out_commitments: Vec<Bn254Fr>,
	out_pub_keys: Vec<Bn254Fr>,
) -> (
	VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		PoseidonCRH_x5_4<Bn254Fr>,
		PoseidonCRH_x5_4Gadget<Bn254Fr>,
		PoseidonCRH_x5_5<Bn254Fr>,
		PoseidonCRH_x5_5Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
	Vec<Bn254Fr>,
) {
	let chain_id = Bn254Fr::from(0u32);
	let ext_data_hash = arbitrary_data.ext_data.clone();
	let (params2, _, params4, params5) = get_hash_params::<ark_bn254::Fr>(Curve::Bn254);

	let circuit = VACircuit::<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		PoseidonCRH_x5_4<Bn254Fr>,
		PoseidonCRH_x5_4Gadget<Bn254Fr>,
		PoseidonCRH_x5_5<Bn254Fr>,
		PoseidonCRH_x5_5Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>::new(
		public_amount,
		arbitrary_data,
		in_leaf_privates,
		in_keypairs,
		in_leaf_public,
		in_set_private_inputs,
		in_root_set,
		params2,
		params4,
		params5,
		in_paths,
		in_indicies,
		in_nullifiers.clone(),
		out_commitments.clone(),
		out_leaf_privates,
		out_leaf_publics,
		out_pub_keys,
	);

	let mut public_inputs = Vec::new();
	public_inputs.push(chain_id);
	public_inputs.push(public_amount);
	for root in in_root_set {
		public_inputs.push(root);
	}
	for nh in in_nullifiers {
		public_inputs.push(nh);
	}
	for out_cm in out_commitments {
		public_inputs.push(out_cm);
	}
	public_inputs.push(ext_data_hash);
	(circuit, public_inputs)
}

fn setup_keys(
	circuit: VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		PoseidonCRH_x5_4<Bn254Fr>,
		PoseidonCRH_x5_4Gadget<Bn254Fr>,
		PoseidonCRH_x5_5<Bn254Fr>,
		PoseidonCRH_x5_5Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
) -> (Vec<u8>, Vec<u8>) {
	let rng = &mut thread_rng();
	let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), rng).unwrap();

	let mut pk_bytes = Vec::new();
	let mut vk_bytes = Vec::new();
	pk.serialize(&mut pk_bytes).unwrap();
	vk.serialize(&mut vk_bytes).unwrap();
	(pk_bytes, vk_bytes)
}

fn prove(
	circuit: VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		PoseidonCRH_x5_4<Bn254Fr>,
		PoseidonCRH_x5_4Gadget<Bn254Fr>,
		PoseidonCRH_x5_5<Bn254Fr>,
		PoseidonCRH_x5_5Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
	pk_bytes: Vec<u8>,
) -> Vec<u8> {
	let rng = &mut thread_rng();
	let pk = ProvingKey::<ark_bn254::Bn254>::deserialize(&*pk_bytes).unwrap();

	let proof = Groth16::prove(&pk, circuit, rng).unwrap();
	let mut proof_bytes = Vec::new();
	proof.serialize(&mut proof_bytes).unwrap();
	proof_bytes
}

pub fn setup_keypairs(n: usize) -> Vec<Keypair<Bn254Fr, PoseidonCRH_x5_2<Bn254Fr>>> {
	let rng = &mut thread_rng();

	let mut keypairs = Vec::new();
	for _ in 0..n {
		let priv_key = Bn254Fr::rand(rng);
		let keypair = Keypair::<_, PoseidonCRH_x5_2<Bn254Fr>>::new(priv_key);
		keypairs.push(keypair);
	}
	keypairs
}

pub fn setup_leaves(
	chain_ids: &Vec<Bn254Fr>,
	amounts: &Vec<Bn254Fr>,
	keypairs: &Vec<Keypair<Bn254Fr, PoseidonCRH_x5_2<Bn254Fr>>>,
) -> (
	Vec<Bn254Fr>,
	Vec<Bn254Fr>,
	Vec<LeafPrivateInput<Bn254Fr>>,
	Vec<LeafPublicInput<Bn254Fr>>,
) {
	let rng = &mut thread_rng();
	let (params2, _, params4, params5) = get_hash_params::<ark_bn254::Fr>(Curve::Bn254);

	let num_inputs = amounts.len();

	let mut leaves = Vec::new();
	let mut nullifiers = Vec::new();
	let mut private_inputs = Vec::new();
	let mut public_inputs = Vec::new();

	for i in 0..num_inputs {
		let chain_id = Bn254Fr::from(chain_ids[i]);
		let amount = Bn254Fr::from(amounts[i]);
		let blinding = Bn254Fr::rand(rng);
		let index = Bn254Fr::from(i as u64);

		let private_input = LeafPrivateInput::<Bn254Fr>::new(amount, blinding);
		let public_input = LeafPublicInput::<Bn254Fr>::new(chain_id);

		let pub_key = keypairs[i].public_key(&params2).unwrap();

		let leaf = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_leaf(
			&private_input,
			&pub_key,
			&public_input,
			&params5,
		)
		.unwrap();

		let signature = keypairs[i]
			.signature::<PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>(&leaf, &index, &params4)
			.unwrap();

		let nullfier = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_nullifier(
			&signature, &leaf, &params4, &index,
		)
		.unwrap();

		leaves.push(leaf);
		nullifiers.push(nullfier);
		private_inputs.push(private_input);
		public_inputs.push(public_input);
	}

	(leaves, nullifiers, private_inputs, public_inputs)
}

pub fn setup_tree(leaves: &Vec<Bn254Fr>) -> (Vec<Path<TreeConfig_x5<Bn254Fr>, TREE_DEPTH>>, Vec<Bn254Fr>, Bn254Fr) {
	let (_, params3, ..) = get_hash_params::<ark_bn254::Fr>(Curve::Bn254);
	let inner_params = Rc::new(params3.clone());
	let tree = Tree_x5::new_sequential(inner_params, Rc::new(()), &leaves).unwrap();
	let root = tree.root();

	let num_leaves = leaves.len();

	let mut paths = Vec::new();
	let mut indices = Vec::new();
	for i in 0..num_leaves {
		let path = tree.generate_membership_proof::<TREE_DEPTH>(i as u64);
		let index = path.get_index(&root, &leaves[i]).unwrap();
		paths.push(path);
		indices.push(index);
	}

	(paths, indices, root.inner())
}

pub fn setup_root_set(root: Bn254Fr) -> ([Bn254Fr; M], Vec<SetPrivateInputs<Bn254Fr, M>>) {
	let root_set = [root.clone(); M];

	let mut set_private_inputs = Vec::new();
	for _ in 0..M {
		let set_private_input = SetMembership::generate_secrets(&root, &root_set).unwrap();
		set_private_inputs.push(set_private_input);
	}

	(root_set, set_private_inputs)
}

pub fn setup_tree_and_set(
	leaves: &Vec<Bn254Fr>,
) -> (
	Vec<Path<TreeConfig_x5<Bn254Fr>, TREE_DEPTH>>,
	Vec<Bn254Fr>,
	[Bn254Fr; M],
	Vec<SetPrivateInputs<Bn254Fr, M>>,
) {
	let (paths, indices, root) = setup_tree(&leaves);
	let (root_set, set_private_inputs) = setup_root_set(root);
	(paths, indices, root_set, set_private_inputs)
}

pub fn setup_arbitrary_data(ext_data: Bn254Fr) -> VAnchorArbitraryData<Bn254Fr> {
	VAnchorArbitraryData::new(ext_data)
}

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[12..].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
