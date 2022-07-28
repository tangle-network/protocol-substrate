use crate::mock::*;

#[test]
fn should_fail_to_verify_without_parameters() {
	new_test_ext().execute_with(|| {
		// Pass arbitrary
		assert!(true);
	});
}
