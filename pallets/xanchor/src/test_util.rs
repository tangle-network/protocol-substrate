use ark_bn254::Bn254;
use ark_ff::{BigInteger, PrimeField};
use arkworks_native_gadgets::poseidon::Poseidon;
use arkworks_setups::{
	common::{setup_params, setup_tree_and_create_path, AnchorProof, Leaf},
	r1cs::anchor::AnchorR1CSProver,
	AnchorProver, Curve,
};
use arkworks_setups::common::{keccak_256, prove, setup_keys, verify};
use webb_primitives::ElementTrait;

use wasm_utils::{
	proof::{generate_proof_js, AnchorProofInput, JsProofInput, ProofInput},
	types::{Backend, Curve as WasmCurve},
};

use crate::mock::parachain::Element;
use codec::Encode;
use arkworks_native_gadgets::poseidon::FieldHasher;
use arkworks_r1cs_circuits::anchor::AnchorCircuit;
use arkworks_r1cs_gadgets::poseidon::PoseidonGadget;
use ark_bn254::Fr;


type Bn254Fr = ark_bn254::Fr;

type ProofBytes = Vec<u8>;
type RootsElement = Vec<Element>;
type NullifierHashElement = Element;
type LeafElement = Element;

pub const DEFAULT_LEAF: [u8; 32] = [0u8; 32];
// merkle proof path legth
// TreeConfig_x5, x7 HEIGHT is hardcoded to 30
pub const TREE_DEPTH: usize = 30;
pub const ANCHOR_CT: usize = 2;
pub type AnchorR1CSProver_Bn254_30_2 = AnchorR1CSProver<Bn254, TREE_DEPTH, ANCHOR_CT>;

pub fn setup_leaf(curve: Curve, chain_id: u64) -> (
	Element, // Secret
	Element, // Nullifier
	Element, // Leaf
	Element  // Nullifier Hash
) {
	let rng = &mut ark_std::test_rng();

	match curve {
		Curve::Bn254 => {
			/*let (secret, nullifier, leaf, nullifier_hash) =
				setup_leaf_x5_4::<Bn254Fr, _>(Curve::Bn254, chain_id.into(), rng).unwrap();*/
			let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes, .. } =
				AnchorR1CSProver_Bn254_30_2::create_random_leaf(Curve::Bn254, chain_id, rng)
					.unwrap();

			let secret_element = Element::from_bytes(&secret_bytes);
			let nullifier_element = Element::from_bytes(&nullifier_bytes);
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash_bytes);
			let leaf_element = Element::from_bytes(&leaf_bytes);

			(secret_element, nullifier_element, leaf_element, nullifier_hash_element)
		}
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}

pub fn 	setup_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	commitment_bytes: Vec<u8>,
	pk_bytes: Vec<u8>,
	chain_id: u64,
	secret: Element,
	nullifier: Element,
	nullifier_hash_bytes: Element,
	leaves: Vec<Element>,
	roots: [Element; 2],
	fee_value: u128,
	refund_value: u128,
) -> (ProofBytes, RootsElement, NullifierHashElement) {
	let rng = &mut ark_std::test_rng();

	match curve {
		Curve::Bn254 => {
			/*let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes, .. } =
				AnchorR1CSProver_Bn254_30_2::create_random_leaf(Curve::Bn254, chain_id.into(), rng)
					.unwrap();*/
			//let leaves = vec![leaf_bytes.clone()];
			//let leaves_f = vec![Bn254Fr::from_le_bytes_mod_order(&leaf_bytes)];
			let leaves_bytes = leaves.iter().map(|x| x.to_vec()).collect();
			let roots_bytes = roots.map(|x| x.to_vec());

			println!("roots_bytes here {:?}", roots_bytes);
			println!("leaves_bytes here {:?}", leaves_bytes);

			let index = 0;

			let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
			let poseidon3 = Poseidon::new(params3);
			/*let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
				&poseidon3,
				&leaves_f,
				index,
				&DEFAULT_LEAF,
			)
				.unwrap();*/
			//let roots_f = [tree.root(); ANCHOR_CT];
			//let roots_raw = roots_f.map(|x| x.into_repr().to_bytes_le());

			let AnchorProof { proof, roots_raw, public_inputs_raw, .. } =
				AnchorR1CSProver_Bn254_30_2::create_proof(
					curve,
					chain_id,
					secret.to_vec(),
					nullifier.to_vec(),
					leaves_bytes,
					index,
					roots_bytes,
					recipient_bytes,
					relayer_bytes,
					fee_value,
					refund_value,
					commitment_bytes,
					pk_bytes,
					DEFAULT_LEAF,
					rng,
				)
					.unwrap();

			let roots_element = roots_raw.iter().map(|x| Element::from_bytes(&x)).collect();
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash_bytes.to_vec());
			//let leaf_element = leaves;

			(proof, roots_element, nullifier_hash_element)
		},
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}

pub fn setup_2_anchors_proof(chain_id_u64_first_anchor: u64,
							 chain_id_u64_second_anchor: u64,
							 roots: [Vec<Element>; ANCHOR_CT]) -> bool {
	let rng = &mut ark_std::test_rng();
	let curve = Curve::Bn254;

	let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
	let params4 = setup_params::<Bn254Fr>(curve, 5, 4);

	let tree_hasher = Poseidon::<Bn254Fr> { params: params3 };
	let leaf_hasher = Poseidon::<Bn254Fr> { params: params4 };

	// setup chain id for first anchor
	let chain_id_first_anchor = Bn254Fr::from(chain_id_u64_first_anchor);


	// setup chain id for second anchor
	let chain_id_second_anchor = Bn254Fr::from(chain_id_u64_second_anchor);

	// setup leaf, secret, nullifier and leaves for first anchor
	// make the leaf you insert into the first anchor have the chain ID of the
	// chain_id_u64_second_anchor
	let leaf_first_anchor = AnchorR1CSProver_Bn254_30_2::create_random_leaf(
		curve,
		chain_id_u64_second_anchor,
		rng,
	)
		.unwrap();
	let secret_first_anchor = Bn254Fr::from_le_bytes_mod_order(&leaf_first_anchor.secret_bytes);
	let nullifier_first_anchor =
		Bn254Fr::from_le_bytes_mod_order(&leaf_first_anchor.nullifier_bytes);
	let leaves_first_anchor = vec![Bn254Fr::from_le_bytes_mod_order(
		&leaf_first_anchor.leaf_bytes,
	)];

	// setup leaf, secret, nullifier and leaves for second anchor
	let leaf_second_anchor = AnchorR1CSProver_Bn254_30_2::create_random_leaf(
		curve,
		chain_id_u64_second_anchor,
		rng,
	)
		.unwrap();
	let secret_second_anchor = Bn254Fr::from_le_bytes_mod_order(&leaf_second_anchor.secret_bytes);
	let nullifier_second_anchor =
		Bn254Fr::from_le_bytes_mod_order(&leaf_second_anchor.nullifier_bytes);
	let leaves_second_anchor = vec![Bn254Fr::from_le_bytes_mod_order(
		&leaf_second_anchor.leaf_bytes,
	)];

	// nullifier hash for first anchor
	let nullifier_hash_first_anchor = tree_hasher
		.hash_two(&nullifier_first_anchor, &nullifier_first_anchor)
		.unwrap();

	// nullifier hash for second anchor
	let nullifier_hash_second_anchor = tree_hasher
		.hash_two(&nullifier_second_anchor, &nullifier_second_anchor)
		.unwrap();

	let index = 0;

	// sets up a merkle tree and generates path for it
	// tree for first anchor
	let (tree_first_anchor, path_first_anchor) =
		setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
			&tree_hasher,
			&leaves_first_anchor,
			index,
			&DEFAULT_LEAF,
		)
			.unwrap();

	// tree for second anchor
	let (tree_second_anchor, path_second_anchor) =
		setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
			&tree_hasher,
			&leaves_second_anchor,
			index,
			&DEFAULT_LEAF,
		)
			.unwrap();

	// roots for first anchor (m_1, m_2)
	let mut roots_first_anchor = [Bn254Fr::from(0u64); ANCHOR_CT];
	//roots_first_anchor[0] = tree_first_anchor.root();
	//roots_first_anchor[1] = tree_second_anchor.root();

	roots_first_anchor =  roots.map(|x| E::Fr::from_le_bytes_mod_order(&x));

	// roots for second anchor (m_2, m_1)
	let mut roots_second_anchor = [Bn254Fr::from(0u64); ANCHOR_CT];
	roots_second_anchor[0] = tree_second_anchor.root();
	roots_second_anchor[1] = tree_first_anchor.root();

	// config for arbitrary input
	let commitment = vec![0u8; 32];
	let recipient = vec![0u8; 32];
	let relayer = vec![0u8; 32];
	let fee = 0u128;
	let refund = 0u128;

	// Create the arbitrary input data
	let mut arbitrary_data_bytes = Vec::new();
	arbitrary_data_bytes.extend(&recipient);
	arbitrary_data_bytes.extend(&relayer);
	// Using encode to be compatible with on chain types
	arbitrary_data_bytes.extend(fee.encode());
	arbitrary_data_bytes.extend(refund.encode());
	arbitrary_data_bytes.extend(&commitment);
	let arbitrary_data = keccak_256(&arbitrary_data_bytes);
	let arbitrary_input = Bn254Fr::from_le_bytes_mod_order(&arbitrary_data);

	// create a circuit for the second anchor
	// using the leaf secret values of the deposit in the first anchor
	// pass in the chain ID and root set of the second anchor
	let anchor_circuit_second_anchor =
		AnchorCircuit::<Bn254Fr, PoseidonGadget<Bn254Fr>, TREE_DEPTH, ANCHOR_CT>::new(
			arbitrary_input,
			secret_first_anchor,
			nullifier_first_anchor,
			chain_id_second_anchor,
			roots_second_anchor,
			path_first_anchor,
			nullifier_hash_first_anchor,
			tree_hasher,
			leaf_hasher,
		);

	let public_inputs_second_anchor = AnchorR1CSProver_Bn254_30_2::construct_public_inputs(
		chain_id_second_anchor,
		nullifier_hash_first_anchor,
		roots_second_anchor,
		arbitrary_input,
	);

	let (pk_second_anchor, vk_second_anchor) =
		setup_keys::<Bn254, _, _>(anchor_circuit_second_anchor.clone(), rng).unwrap();

	let proof = prove::<Bn254, _, _>(anchor_circuit_second_anchor, &pk_second_anchor, rng).unwrap();
	let res = verify::<Bn254>(&public_inputs_second_anchor, &vk_second_anchor, &proof).unwrap();


	//assert_eq!(res, true);

	res
}

pub fn setup_wasm_utils_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	commitment_bytes: [u8; 32],
	pk_bytes: Vec<u8>,
	chain_id: u64,
	fee_value: u128,
	refund_value: u128,
	roots_raw:[Vec<u8>; 2],
) -> (
	Vec<u8>,      // proof bytes
	Vec<Element>, // roots
	Element,      // nullifier_hash
	Element,      // leaf
) {
	match curve {
		Curve::Bn254 => {
			let note_secret = "7e0f4bfa263d8b93854772c94851c04b3a9aba38ab808a8d081f6f5be9758110b7147c395ee9bf495734e4703b1f622009c81712520de0bbd5e7a10237c7d829bf6bd6d0729cca778ed9b6fb172bbb12b01927258aca7e0a66fd5691548f8717";
			let raw = hex::decode(&note_secret).unwrap();

			let secret = &raw[0..32];
			let nullifier = &raw[0..64];
			let leaf = AnchorR1CSProver_Bn254_30_2::create_leaf_with_privates(
				curve,
				chain_id,
				secret.to_vec(),
				nullifier.to_vec(),
			)
				.unwrap();

			let leaves = vec![leaf.leaf_bytes.clone()];
			let leaves_f = vec![Bn254Fr::from_le_bytes_mod_order(&leaf.leaf_bytes)];
			let index = 0;

			let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
			let poseidon3 = Poseidon::new(params3);

			let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
				&poseidon3,
				&leaves_f,
				index,
				&DEFAULT_LEAF,
			)
				.unwrap();
			//let roots_f = [tree.root(); ANCHOR_CT];
			//let roots_raw = roots_f.map(|x| x.into_repr().to_bytes_le());

			let mixer_proof_input = AnchorProofInput {
				exponentiation: 5,
				width: 4,
				curve: WasmCurve::Bn254,
				backend: Backend::Arkworks,
				secret: secret.to_vec(),
				nullifier: nullifier.to_vec(),
				recipient: recipient_bytes,
				relayer: relayer_bytes,
				pk: pk_bytes,
				refund: refund_value,
				fee: fee_value,
				chain_id,
				leaves,
				leaf_index: index,
				roots: roots_raw.to_vec(),
				refresh_commitment: commitment_bytes,
			};
			let js_proof_inputs = JsProofInput { inner: ProofInput::Anchor(mixer_proof_input) };
			let proof = generate_proof_js(js_proof_inputs).unwrap();

			let root_elements = proof.roots.iter().map(|root| Element::from_bytes(&root)).collect();
			let nullifier_hash_element = Element::from_bytes(&proof.nullifier_hash);
			let leaf_element = Element::from_bytes(&proof.leaf);

			(proof.proof, root_elements, nullifier_hash_element, leaf_element)
		},
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[12..].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
