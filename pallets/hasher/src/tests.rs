use super::*;
use crate::mock::*;
use ark_ff::prelude::*;
use arkworks_utils::utils::common::{setup_params_x5_3, Curve};
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
	type Fr = ark_bn254::Fr;
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params_x5_3::<Fr>(curve);
		let res = BN254CircomPoseidon3x5Hasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
	});
}

#[test]
fn should_output_correct_hash() {
	type Fr = ark_bn254::Fr;
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params_x5_3::<Fr>(curve);
		let res = BN254CircomPoseidon3x5Hasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
		let left = Fr::one().into_repr().to_bytes_le(); // one
		let right = Fr::one().double().into_repr().to_bytes_le(); // two
		let hash = BN254CircomPoseidon3x5Hasher::hash_two(&left, &right).unwrap();
		let f = Fr::from_le_bytes_mod_order(&hash).into_repr().to_bytes_be();
		assert_eq!(
			f,
			bytes::from_hex("0x115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a").unwrap()
		);
	});
}
