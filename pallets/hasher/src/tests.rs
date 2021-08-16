use super::*;
use crate::mock::*;
use ark_ff::prelude::*;
use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
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
		let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
		let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
		let params = PoseidonParameters::new(rounds, mds);
		let res = BN254CircomPoseidon3x5Hasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
	});
}

#[test]
fn should_output_correct_hash() {
	type Fq = ark_bn254::Fq;
	new_test_ext().execute_with(|| {
		let rounds = get_rounds_poseidon_circom_bn254_x5_3::<Fq>();
		let mds = get_mds_poseidon_circom_bn254_x5_3::<Fq>();
		let params = PoseidonParameters::new(rounds, mds);
		let res = BN254CircomPoseidon3x5Hasher::force_set_parameters(Origin::root(), params.to_bytes());
		assert_ok!(res);
		let left = Fq::one().into_repr().to_bytes_be(); // one
		let right = Fq::one().double().into_repr().to_bytes_be(); // two
		let hash = BN254CircomPoseidon3x5Hasher::hash_two(&left, &right);
		assert_ok!(
			hash,
			bytes::from_hex("0x115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a").unwrap()
		);
	});
}
