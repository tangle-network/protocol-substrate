use super::mock::*;
use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
use frame_support::{assert_ok, traits::OnInitialize};

const TEST_MAX_EDGES: u32 = 30;

fn hasher_params() -> Vec<u8> {
	let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let params = PoseidonParameters::new(rounds, mds);
	params.to_bytes()
}

#[test]
fn should_create_anchor() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		assert_ok!(Anchor::create(Origin::root(), TEST_MAX_EDGES, 3, 0));
	});
}
