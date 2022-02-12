use ark_bn254::Bn254;
use ark_ff::{BigInteger, FromBytes, PrimeField};
pub use arkworks_circuits::setup::{
	common::{prove, prove_unchecked, verify_unchecked_raw},
	mixer::{
		setup_leaf_with_privates_raw_x5_5, setup_leaf_x5_5, setup_proof_x5_5, MixerProverSetup,
	},
};
use arkworks_utils::utils::common::Curve;

// wasm-utils dependencies
use wasm_utils::{
	proof::{generate_proof_js, JsProofInput, MixerProofInput, ProofInput},
	types::{Backend, Curve as WasmCurve},
};

use super::Element;

use ark_bn254::Fr as Bn254Fr;

pub fn setup_wasm_utils_zk_circuit(
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	pk_bytes: Vec<u8>,
	fee_value: u128,
	refund_value: u128,
) -> (
	Vec<u8>, // proof bytes
	Element, // root
	Element, // nullifier_hash
	Element, // leaf
) {
    let note_secret = "7e0f4bfa263d8b93854772c94851c04b3a9aba38ab808a8d081f6f5be9758110b7147c395ee9bf495734e4703b1f622009c81712520de0bbd5e7a10237c7d829bf6bd6d0729cca778ed9b6fb172bbb12b01927258aca7e0a66fd5691548f8717";

    let secret = hex::decode(&note_secret[0..32]).unwrap();
    let nullifier = hex::decode(&note_secret[32..64]).unwrap();
    let (leaf, _) = setup_leaf_with_privates_raw_x5_5::<Bn254Fr>(
        Curve::Bn254,
        secret.clone(),
        nullifier.clone(),
    )
    .unwrap();

    let leaves = vec![leaf];

    let mixer_proof_input = MixerProofInput {
        exponentiation: 5,
        width: 5,
        curve: WasmCurve::Bn254,
        backend: Backend::Arkworks,
        secrets: secret,
        nullifier,
        recipient: recipient_bytes,
        relayer: relayer_bytes,
        pk: pk_bytes,
        refund: refund_value,
        fee: fee_value,
        chain_id: 0,
        leaves,
        leaf_index: 0,
    };
    let js_proof_inputs = JsProofInput { inner: ProofInput::Mixer(mixer_proof_input) };
    let proof = generate_proof_js(js_proof_inputs).unwrap();

    let root_array: [u8; 32] = proof.root.try_into().unwrap();
    let root_element = Element(root_array);

    let nullifier_hash_array: [u8; 32] = proof.nullifier_hash.try_into().unwrap();
    let nullifier_hash_element = Element(nullifier_hash_array);

    let leaf_array: [u8; 32] = proof.leaf.try_into().unwrap();
    let leaf_element = Element(leaf_array);

    (proof.proof, root_element, nullifier_hash_element, leaf_element)
}