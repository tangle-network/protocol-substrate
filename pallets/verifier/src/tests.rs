use crate::{mock::*};


#[test]
fn verify_nothing_with_test_verifier() {
	new_test_ext().execute_with(|| {
		// Read pallet storage and assert an expected result.
		assert_eq!(<VerifierPallet as VerifierModule>::hash(&[1u8; 32]), [1u8; 32]);
	});
}
