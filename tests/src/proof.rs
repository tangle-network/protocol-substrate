use ark_ff::{PrimeField, BigInteger};
pub use arkworks_circuits::setup::{
	common::{prove, prove_unchecked, verify_unchecked_raw},
    mixer, anchor,
};
use arkworks_utils::utils::common::Curve;

// wasm-utils dependencies
use wasm_utils::{
	proof::{generate_proof_js, JsProofInput, MixerProofInput, AnchorProofInput, ProofInput},
	types::{Backend, Curve as WasmCurve},
};
use ark_std::rand::thread_rng;
use ark_std::UniformRand;

use super::Element;

use ark_bn254::Fr as Bn254Fr;

pub fn setup_mixer_leaf() -> (Element, Element, Element, Element) {
    let rng = &mut thread_rng();
    let secret = Bn254Fr::rand(rng).into_repr().to_bytes_le();
    let nullifier = Bn254Fr::rand(rng).into_repr().to_bytes_le();
    let (leaf, nullifier_hash) = mixer::setup_leaf_with_privates_raw_x5_5::<Bn254Fr>(
        Curve::Bn254,
        secret.clone(),
        nullifier.clone(),
    )
    .unwrap();

    let leaf_array: [u8; 32] = leaf.try_into().unwrap();
    let leaf_element = Element(leaf_array);

    let secret_array: [u8; 32] = secret.try_into().unwrap();
    let secret_element = Element(secret_array);

    let nullifier_array: [u8; 32] = nullifier.try_into().unwrap();
    let nullifier_element = Element(nullifier_array);

    let nullifier_hash_array: [u8; 32] = nullifier_hash.try_into().unwrap();
    let nullifier_hash_element = Element(nullifier_hash_array);
    
    (leaf_element, secret_element, nullifier_element, nullifier_hash_element)
}

pub fn setup_anchor_leaf(chain_id: u128) -> (Element, Element, Element, Element) {
    let rng = &mut thread_rng();
    let secret = Bn254Fr::rand(rng).into_repr().to_bytes_le();
    let nullifier = Bn254Fr::rand(rng).into_repr().to_bytes_le();
    let (leaf, nullifier_hash) = anchor::setup_leaf_with_privates_raw_x5_4::<Bn254Fr>(
        Curve::Bn254,
        secret.clone(),
        nullifier.clone(),
        chain_id
    )
    .unwrap();

    let leaf_array: [u8; 32] = leaf.try_into().unwrap();
    let leaf_element = Element(leaf_array);

    let secret_array: [u8; 32] = secret.try_into().unwrap();
    let secret_element = Element(secret_array);

    let nullifier_array: [u8; 32] = nullifier.try_into().unwrap();
    let nullifier_element = Element(nullifier_array);

    let nullifier_hash_array: [u8; 32] = nullifier_hash.try_into().unwrap();
    let nullifier_hash_element = Element(nullifier_hash_array);
    
    (leaf_element, secret_element, nullifier_element, nullifier_hash_element)
}

pub fn setup_mixer_circuit(
    leaves: Vec<Vec<u8>>,
    leaf_index: u64,
    secret: Vec<u8>,
    nullifier: Vec<u8>,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	fee_value: u128,
	refund_value: u128,
    pk_bytes: Vec<u8>,
) -> (
	Vec<u8>, // proof bytes
	Element, // root
) {
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
        leaf_index,
    };
    let js_proof_inputs = JsProofInput { inner: ProofInput::Mixer(mixer_proof_input) };
    let proof = generate_proof_js(js_proof_inputs).unwrap();

    let root_array: [u8; 32] = proof.root.try_into().unwrap();
    let root_element = Element(root_array);

    (proof.proof, root_element)
}

pub fn setup_anchor_circuit(
    roots: Vec<Vec<u8>>,
    leaves: Vec<Vec<u8>>,
    leaf_index: u64,
    chain_id: u128,
    secret: Vec<u8>,
    nullifier: Vec<u8>,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	fee_value: u128,
	refund_value: u128,
    commitment_bytes: Vec<u8>,
    pk_bytes: Vec<u8>,
) -> (
	Vec<u8>, // proof bytes
	Element, // root
) {
    let commitment: [u8; 32] = commitment_bytes.try_into().unwrap();
    let mixer_proof_input = AnchorProofInput {
        exponentiation: 5,
        width: 4,
        curve: WasmCurve::Bn254,
        backend: Backend::Arkworks,
        secrets: secret,
        nullifier,
        recipient: recipient_bytes,
        relayer: relayer_bytes,
        pk: pk_bytes,
        refund: refund_value,
        fee: fee_value,
        chain_id,
        leaves,
        leaf_index,
        roots,
        commitment
    };
    let js_proof_inputs = JsProofInput { inner: ProofInput::Anchor(mixer_proof_input) };
    let proof = generate_proof_js(js_proof_inputs).unwrap();

    let root_array: [u8; 32] = proof.root.try_into().unwrap();
    let root_element = Element(root_array);

    (proof.proof, root_element)
}