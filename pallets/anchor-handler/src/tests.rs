use crate::{mock::*, Error};
use frame_support::assert_err;
use sp_runtime::traits::BadOrigin;

#[test]
fn set_maintainer() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_err!(AnchorHandler::set_maintainer(Origin::signed(1), 42), BadOrigin);
	});
}