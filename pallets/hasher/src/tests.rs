use super::*;
use crate::mock::*;
use frame_support::{assert_err, instances::Instance1};

#[test]
fn hash_nothing_with_test_hasher() {
	new_test_ext().execute_with(|| {
		// Read pallet storage and assert an expected result.
		assert_err!(
			<HasherPallet as HasherModule>::hash(&[1u8; 32]),
			Error::<Test, Instance1>::ParametersNotInitialized
		);
	});
}
