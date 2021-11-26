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
	set::membership::SetMembership,
	setup::common::{
		setup_params_x5_2, setup_params_x5_3, setup_params_x5_4, setup_params_x5_5, Curve, LeafCRH, LeafCRHGadget,
		PoseidonCRH_x5_2, PoseidonCRH_x5_2Gadget, PoseidonCRH_x5_3Gadget, PoseidonCRH_x5_4, PoseidonCRH_x5_4Gadget,
		PoseidonCRH_x5_5, PoseidonCRH_x5_5Gadget, TreeConfig_x5, Tree_x5,
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

pub fn get_hash_params<T: PrimeField>(curve: Curve) -> (Vec<u8>, Vec<u8>) {
	(
		setup_params_x5_3::<T>(curve).to_bytes(),
		setup_params_x5_5::<T>(curve).to_bytes(),
	)
}

fn setup_random_circuit() -> (Vec<u8>, Vec<u8>) {
	let rng = &mut thread_rng();
	let hasher_params_w2 = setup_params_x5_2(Curve::Bn254);
	let hasher_params_w3 = setup_params_x5_3::<Bn254Fr>(Curve::Bn254);
	let hasher_params_w4 = setup_params_x5_4(Curve::Bn254);
	let hasher_params_w5 = setup_params_x5_5(Curve::Bn254);

	let chain_id = Bn254Fr::rand(rng);

	// TODO: hash them with keccak
	let recipient = Bn254Fr::rand(rng);
	let relayer = Bn254Fr::rand(rng);
	let fee = Bn254Fr::rand(rng);
	let refund = Bn254Fr::rand(rng);

	let in_amount_1 = Bn254Fr::rand(rng);
	let in_amount_2 = Bn254Fr::rand(rng);

	let blinding_1 = Bn254Fr::rand(rng);
	let blinding_2 = Bn254Fr::rand(rng);

	let private_key_1 = Bn254Fr::rand(rng);
	let private_key_2 = Bn254Fr::rand(rng);

	let public_amount = Bn254Fr::rand(rng);
	let ext_data_hash = Bn254Fr::rand(rng);

	let out_chain_id_1 = Bn254Fr::rand(rng);
	let out_amount_1 = Bn254Fr::rand(rng);
	let out_pubkey_1 = Bn254Fr::rand(rng);
	let out_blinding_1 = Bn254Fr::rand(rng);

	let out_chain_id_2 = Bn254Fr::rand(rng);
	let out_amount_2 = Bn254Fr::rand(rng);
	let out_pubkey_2 = Bn254Fr::rand(rng);
	let out_blinding_2 = Bn254Fr::rand(rng);

	let leaf_private_1 = LeafPrivateInput::<Bn254Fr>::new(in_amount_1, blinding_1);
	let leaf_private_2 = LeafPrivateInput::<Bn254Fr>::new(in_amount_2, blinding_2);
	let leaf_public_input = LeafPublicInput::<Bn254Fr>::new(chain_id.clone());

	let keypair_1 = Keypair::<_, PoseidonCRH_x5_2<Bn254Fr>>::new(private_key_1.clone());
	let public_key_1 = keypair_1.public_key(&hasher_params_w2).unwrap();
	let keypair_2 = Keypair::new(private_key_2.clone());
	let public_key_2 = keypair_2.public_key(&hasher_params_w2).unwrap();

	let leaf_1 = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_leaf(
		&leaf_private_1,
		&public_key_1,
		&leaf_public_input,
		&hasher_params_w5,
	)
	.unwrap();
	let leaf_2 = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_leaf(
		&leaf_private_2,
		&public_key_2,
		&leaf_public_input,
		&hasher_params_w5,
	)
	.unwrap();
	let leaves = [leaf_1, leaf_2];

	let inner_params = Rc::new(hasher_params_w3.clone());
	let tree = Tree_x5::new_sequential(inner_params, Rc::new(()), &leaves).unwrap();

	let path_1 = tree.generate_membership_proof::<TREE_DEPTH>(0);
	let path_2 = tree.generate_membership_proof::<TREE_DEPTH>(1);
	let root = tree.root().inner();
	let mut root_set = [Bn254Fr::rand(rng); M];
	root_set[0] = root;
	let index_0: Bn254Fr = path_1.get_index(&tree.root(), &leaf_1).unwrap();
	let index_1: Bn254Fr = path_1.get_index(&tree.root(), &leaf_2).unwrap();

	let vanchor_arbitrary_data = VAnchorArbitraryData::new(ext_data_hash);

	let signature = keypair_1
		.signature::<PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>(&leaf_1, &index_0, &hasher_params_w4)
		.unwrap();
	let nullifier_hash_1 = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_nullifier(
		&signature,
		&leaf_1,
		&hasher_params_w4,
		&index_0,
	)
	.unwrap();
	let signature = keypair_2
		.signature::<PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>(&leaf_2, &index_1, &hasher_params_w4)
		.unwrap();
	let nullifier_hash_2 = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_nullifier(
		&signature,
		&leaf_2,
		&hasher_params_w4,
		&index_1,
	)
	.unwrap();

	let set_private_inputs_1 = SetMembership::generate_secrets(&root, &root_set).unwrap();

	let out_leaf_private_1 = LeafPrivateInput::new(out_amount_1, out_blinding_1);
	let out_leaf_private_2 = LeafPrivateInput::<Bn254Fr>::new(out_amount_2, out_blinding_2);

	let out_leaf_public_1 = LeafPublicInput::new(out_chain_id_1);
	let out_leaf_public_2 = LeafPublicInput::new(out_chain_id_2);

	let output_commitment_1 = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_leaf(
		&out_leaf_private_1,
		&out_pubkey_1,
		&out_leaf_public_1,
		&hasher_params_w5,
	)
	.unwrap();

	let output_commitment_2 = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_leaf(
		&out_leaf_private_2,
		&out_pubkey_2,
		&out_leaf_public_2,
		&hasher_params_w5,
	)
	.unwrap();

	let leaf_private_inputs = vec![leaf_private_1.clone(), leaf_private_2.clone()];
	let keypair_inputs = vec![keypair_1.clone(), keypair_2.clone()];
	let paths = vec![path_1.clone(), path_2.clone()];
	let indices = vec![index_0, index_1];
	let nullifier_hashes = vec![nullifier_hash_1, nullifier_hash_2];
	let set_private_inputs = vec![set_private_inputs_1.clone(), set_private_inputs_1.clone()];
	let out_leaf_privates = vec![out_leaf_private_1.clone(), out_leaf_private_2.clone()];
	let out_leaf_publics = vec![out_leaf_public_1.clone(), out_leaf_public_2.clone()];
	let out_pubkeys = vec![out_pubkey_1, out_pubkey_2];
	let output_commitments = vec![output_commitment_1, output_commitment_2];

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
		public_amount.clone(),
		vanchor_arbitrary_data,
		leaf_private_inputs,
		keypair_inputs,
		leaf_public_input,
		set_private_inputs,
		root_set.clone(),
		hasher_params_w2,
		hasher_params_w4,
		hasher_params_w5,
		paths,
		indices,
		nullifier_hashes.clone(),
		output_commitments.clone(),
		out_leaf_privates,
		out_leaf_publics,
		out_pubkeys,
	);

	let mut public_inputs = Vec::new();
	public_inputs.push(chain_id);
	public_inputs.push(public_amount);
	for root in root_set {
		public_inputs.push(root);
	}
	for nh in nullifier_hashes {
		public_inputs.push(nh);
	}
	for out_cm in output_commitments {
		public_inputs.push(out_cm);
	}
	public_inputs.push(ext_data_hash);

	let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), rng).unwrap();

	let mut pk_bytes = Vec::new();
	let mut vk_bytes = Vec::new();
	pk.serialize(&mut pk_bytes).unwrap();
	vk.serialize(&mut vk_bytes).unwrap();
	(pk_bytes, vk_bytes)
}

pub fn setup_default_leaves(
	chain_id: u32,
	amounts: Vec<u32>,
	keypairs: Vec<Keypair<Bn254Fr, PoseidonCRH_x5_2<Bn254Fr>>>,
	params2: PoseidonParameters<Bn254Fr>,
	params5: PoseidonParameters<Bn254Fr>,
) -> (Vec<Bn254Fr>, Vec<LeafPrivateInput<Bn254Fr>>, LeafPublicInput<Bn254Fr>) {
	let rng = &mut thread_rng();

	let chain_id = Bn254Fr::from(0u32);

	let num_inputs = amounts.len();

	let mut leaves = Vec::new();
	let mut private_inputs = Vec::new();

	// Public inputs are reused
	let public_input = LeafPublicInput::<Bn254Fr>::new(chain_id.clone());
	for i in 0..num_inputs {
		let amount = Bn254Fr::from(amounts[i]);
		let blinding = Bn254Fr::rand(rng);

		let private_input = LeafPrivateInput::<Bn254Fr>::new(amount, blinding);

		let pub_key = keypairs[i].public_key(&params2).unwrap();

		let leaf = Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>, PoseidonCRH_x5_5<Bn254Fr>>::create_leaf(
			&private_input,
			&pub_key,
			&public_input,
			&params5,
		)
		.unwrap();

		leaves.push(leaf);
		private_inputs.push(private_input);
	}
	(leaves, private_inputs, public_input)
}

pub fn setup_default_tree(
	leaves: Vec<Bn254Fr>,
	params3: PoseidonParameters<Bn254Fr>,
) -> Vec<Path<TreeConfig_x5<Bn254Fr>, TREE_DEPTH>> {
	let rng = &mut thread_rng();

	let inner_params = Rc::new(params3.clone());
	let tree = Tree_x5::new_sequential(inner_params, Rc::new(()), &leaves).unwrap();

	let num_leaves = leaves.len();

	let mut paths = Vec::new();
	for i in 0..num_leaves {
		let path = tree.generate_membership_proof::<TREE_DEPTH>(i as u64);
		paths.push(path);
	}

	paths
}

pub fn setup_default_root_set(root: Bn254Fr) -> [Bn254Fr; M] {
	let rng = &mut thread_rng();
	let mut root_set = [Bn254Fr::rand(rng); M];
	root_set[0] = root;
	root_set
}

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[12..].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
