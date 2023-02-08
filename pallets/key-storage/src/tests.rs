use super::*;
use crate::mock::*;
use frame_benchmarking::account;
use frame_support::assert_ok;
use webb_primitives::runtime::AccountId;

#[test]
fn should_register_public_key_with_owner() {
	new_test_ext().execute_with(|| {
		let owner = account::<AccountId>("", 0, 0);
		let public_key = [0u8; 32].to_vec();
		let res = KeyStorage::register(
			RuntimeOrigin::signed(owner.clone()),
			owner,
			public_key.try_into().unwrap(),
		);
		assert_ok!(res);
	});
}
