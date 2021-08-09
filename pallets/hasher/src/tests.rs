use super::*;
use crate::mock::*;
use arkworks_gadgets::poseidon::PoseidonParameters;
use frame_support::{assert_err, assert_ok, instances::Instance1};
use sp_core::bytes;

#[test]
fn should_fail_with_params_not_initialized() {
	new_test_ext().execute_with(|| {
		assert_err!(
			<BN254Poseidon3x5Hasher as HasherModule>::hash(&[1u8; 32]),
			Error::<Test, Instance1>::ParametersNotInitialized
		);
	});
}

#[test]
fn should_initialize_parameters() {
	new_test_ext().execute_with(|| {
		let params = PoseidonParameters::<ark_bn254::Fr>::new(
			constants::bn256_3x5::ROUND_KEYS.into(),
			constants::bn256_3x5::MDS_MATRIX.iter().map(|v| v.to_vec()).collect(),
		);
		let res = BN254Poseidon3x5Hasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
	});
}

#[test]
fn should_output_correct_hash() {
	new_test_ext().execute_with(|| {
		let params = PoseidonParameters::new(
			constants::bn256_3x5::ROUND_KEYS.into(),
			constants::bn256_3x5::MDS_MATRIX.iter().map(|v| v.to_vec()).collect(),
		);
		let res = BN254Poseidon3x5Hasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
		let hash = BN254Poseidon3x5Hasher::hash_two(&[1u8; 32], &[2u8; 32]);
		assert_ok!(
			hash,
			bytes::from_hex("0x1acd1ec9914d2b378db637af233324a068ea40e6f26ff34e191bd77816e9810c").unwrap()
		);
	});
}
