#![cfg_attr(not(feature = "std"), no_std)]

pub mod hasher;
pub mod runtime;
pub mod signing;
pub mod traits;
pub mod types;
pub mod utils;
pub mod verifier;

#[cfg(feature = "hashing")]
pub mod hashing;

#[cfg(feature = "verifying")]
pub mod verifying;

#[cfg(feature = "field_ops")]
pub mod field_ops;

pub use hasher::*;
pub use runtime::*;
pub use traits::*;
pub use types::*;
pub use verifier::*;

pub use runtime::*;

pub use webb_proposals;

// /// Common constants of parachains.
// mod constants {
// 	use frame_support::weights::{constants::WEIGHT_PER_SECOND, Weight};
// 	use sp_runtime::Perbill;
// 	/// We assume that ~5% of the block weight is consumed by `on_initialize`
// 	/// handlers. This is used to limit the maximal weight of a single
// 	/// extrinsic.
// 	pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
// 	/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest
// 	/// can be used by Operational  extrinsics.
// 	pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

// 	/// We allow for 0.5 seconds of compute with a 6 second average block time.
// 	pub const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;
// }

/// Opaque types. These are used by the CLI to instantiate machinery that don't
/// need to know the specifics of the runtime. They can then be made to be
/// agnostic over specific formats of data like extrinsics, allowing for them to
/// continue syncing the network through upgrades to even the core data
/// structures.
pub mod opaque {
	use super::*;
	use sp_runtime::{generic, traits::BlakeTwo256};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
}

