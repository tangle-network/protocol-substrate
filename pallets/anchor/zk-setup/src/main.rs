use std::{env, fs, path::Path};

use ark_ff::{BigInteger, FromBytes, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use arkworks_gadgets::{
	prelude::ark_groth16::ProvingKey,
};
use arkworks_circuits::setup::bridge::{
	prove_groth16_circuit_x5, setup_arbitrary_data, setup_groth16_random_circuit_x5,
	setup_leaf_x5, setup_set, Circuit_x5,
};
use arkworks_utils::{
	poseidon::PoseidonParameters,	
	utils::common::{setup_params_x5_3, setup_params_x5_5, setup_tree_and_create_path_tree_x5, Curve},
};
use codec::Encode;

use frame_benchmarking::account;
use sp_runtime::traits::{IdentifyAccount, Verify};

pub const TREE_DEPTH: usize = 30;
pub const M: usize = 2;

const SEED: u32 = 0;
type Bn254Fr = ark_bn254::Fr;
pub type AccountId = <<sp_runtime::MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;

pub fn generate_proofs() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<Vec<u8>>, Vec<u8>, Vec<u8>) {
	let curve = Curve::Bn254;

	let mut pk_bytes = Vec::new();
	let mut vk_bytes = Vec::new();

	let rng = &mut ark_std::test_rng();

	let params = setup_params_x5_3::<Bn254Fr>(curve);

	let (pk, vk) = setup_groth16_random_circuit_x5::<_, ark_bn254::Bn254, TREE_DEPTH, M>(rng, curve);
	vk.serialize(&mut vk_bytes).unwrap();
	pk.serialize(&mut pk_bytes).unwrap();

	// inputs
	let src_chain_id: u32 = 1;
	let mut recipient_account_bytes: Vec<u8> = account::<AccountId>("recipient", 0, SEED).encode()[..20].to_vec();
	let mut relayer_account_bytes: Vec<u8> = account::<AccountId>("relayer", 1, SEED).encode()[..20].to_vec();
	let fee_value: u32 = 0;
	let refund_value: u32 = 0;

	recipient_account_bytes.extend_from_slice(&[0u8; 12]);
	relayer_account_bytes.extend_from_slice(&[0u8; 12]);

	// fit inputs to the curve.
	let chain_id = Bn254Fr::from(src_chain_id);
	let recipient = Bn254Fr::read(&recipient_account_bytes[..]).unwrap();
	let relayer = Bn254Fr::read(&relayer_account_bytes[..]).unwrap();
	let fee = Bn254Fr::from(fee_value);
	let refund = Bn254Fr::from(refund_value);

	let params5 = setup_params_x5_5::<Bn254Fr>(curve);
	let (leaf_private, leaf_public, leaf, nullifier_hash) = setup_leaf_x5(chain_id, &params5, rng);

	// the withdraw process..
	// we setup the inputs to our proof generator.
	let params3 = setup_params_x5_3::<Bn254Fr>(curve);
	let (mt, path) = setup_tree_and_create_path_tree_x5::<_, TREE_DEPTH>(&[leaf], 0, &params3);
	let root = mt.root().inner();

	let mut roots = [Bn254Fr::default(); M];
	roots[0] = root; // local root.

	let set_private_inputs = setup_set(&root, &roots);
	let arbitrary_input = setup_arbitrary_data(recipient, relayer, fee, refund);
	// setup the circuit.
	let circuit = Circuit_x5::new(
		arbitrary_input,
		leaf_private,
		leaf_public,
		set_private_inputs,
		roots,
		params5,
		path,
		root,
		nullifier_hash,
	);
	let pk = ProvingKey::<ark_bn254::Bn254>::deserialize(&*pk_bytes).unwrap();
	// generate the proof.
	let proof = prove_groth16_circuit_x5(&pk, circuit, rng);

	// format the input for the pallet.
	let mut proof_bytes = Vec::new();
	proof.serialize(&mut proof_bytes).unwrap();

	let roots_element_bytes = roots
		.iter()
		.map(|v| v.into_repr().to_bytes_le())
		.collect::<Vec<Vec<u8>>>();

	let nullifier_hash_element_bytes = nullifier_hash.into_repr().to_bytes_le();

	(
		proof_bytes,
		vk_bytes,
		params.to_bytes(),
		roots_element_bytes,
		nullifier_hash_element_bytes,
		leaf.into_repr().to_bytes_le(),
	)
}

fn main() {
	let out_dir = env::var("OUT_DIR").expect("Expected output directory 'OUT_DIR' when running this script");

	let dest_path = Path::new(&out_dir).join("zk_config.rs");
	let (proof_bytes, vk_bytes, hash_params, roots_element_bytes, nullifier_hash_element_bytes, leaf) =
		generate_proofs();

	fs::write(
            &dest_path,
            format!("pub const HASH_PARAMS: [u8;{}] = {:?};\npub const PROOF_BYTES: [u8;{}] = {:?};\npub const VK_BYTES: [u8;{}] = {:?};\npub const ROOT_ELEMENT_BYTES: [[u8;32];{}] = {:?};\npub const NULLIFIER_HASH_ELEMENTS_BYTES: [u8;{}] = {:?};\npub const LEAF: [u8;{}]= {:?};", 
			hash_params.len(), 
			hash_params, 
			proof_bytes.len(), 
			proof_bytes, 
			vk_bytes.len(), 
			vk_bytes, 
			roots_element_bytes.len(), 
			roots_element_bytes, 
			nullifier_hash_element_bytes.len(), 
			nullifier_hash_element_bytes,
			leaf.len(),
			leaf
		)
        ).unwrap();
}
