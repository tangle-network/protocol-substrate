use super::*;
use crate::mock::*;
use frame_support::assert_err;

#[test]
fn should_fail_to_verify_without_parameters() {
	new_test_ext().execute_with(|| {
		// Pass arbitrary
		assert_err!(
			<VerifierPallet as VerifierModule>::verify(&[], &[1u8; 32]),
			Error::<Test, _>::VerifyingParametersNotInitialized
		);
	});
}
