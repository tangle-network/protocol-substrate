use super::*;
use crate::mock::*;
use frame_support::assert_err;

#[test]
fn should_fail_to_verify_without_parameters() {
	new_test_ext().execute_with(|| {
		// Pass arbitrary
		assert_err!(
			<VerifierPallet as VAnchorVerifierModule>::verify(&[], &[1u8; 32], 0, 0),
			Error::<Test, _>::ParametersNotInitialized
		);
	});
}
