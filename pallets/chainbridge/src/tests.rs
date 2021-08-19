use crate::{mock::*, Error};
use frame_support::assert_err;

#[test]
fn set_maintainer() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_err!(
			ChainBridge::set_maintainer(Origin::signed(1), 42),
			Error::<Test>::InvalidPermissions
		);
	});
}
