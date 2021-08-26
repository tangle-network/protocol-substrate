use crate::{mock::*, Error};
use frame_support::assert_err;
use sp_runtime::traits::BadOrigin;

#[test]
fn set_maintainer() {
	new_test_ext().execute_with(|| {
		new_test_ext().execute_with(|| {});
	});
}
