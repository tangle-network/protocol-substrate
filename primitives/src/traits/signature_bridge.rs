//! All the traits exposed to be used in other custom pallets
use frame_support::{dispatch, BoundedVec};

/// Post-processing hook for setting a maintainer after a maintainer is selected
/// in some external process.
pub trait SetMaintainer<N, M> {
	/// Set the maintainer of the pallet.
	fn set_maintainer(
		nonce: N,
		maintainer: BoundedVec<u8, M>,
	) -> Result<(), dispatch::DispatchError>;
}
