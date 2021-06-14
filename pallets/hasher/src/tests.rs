use crate::{mock::*};


#[test]
fn hash_nothing_with_test_hasher() {
	new_test_ext().execute_with(|| {
		// Read pallet storage and assert an expected result.
		assert_eq!(<HasherPallet as HasherModule>::hash(&[1u8; 32]), [1u8; 32]);
	});
}
