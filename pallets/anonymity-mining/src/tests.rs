use super::*;
use crate::mock::*;
use ark_ff::prelude::*;
use arkworks_setups::{common::setup_params, Curve};
use frame_support::{assert_err, assert_ok};
use hex_literal::hex;
use sp_core::bytes;

#[test]
fn should_initialize_parameters() {
	new_test_ext().execute_with(|| {});
}
