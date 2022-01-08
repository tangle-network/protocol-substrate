// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

use frame_support::{dispatch::DispatchResultWithPostInfo, ensure, traits::EnsureOrigin};
use frame_system::pallet_prelude::OriginFor;

///TODO: Define BalanceOf CurrencyIdOf
use pallet_token_wrapper::BalanceOf;

/// Not sure how to import the TokenWrapperInterface in 

use darkwebb_primitives::{
	ResourceId
};

//TODO: should TokenWrapperInterface be moved to primitives/traits
use pallet_token_wrapper::traits::TokenWrapperInterface;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_token_wrapper::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// TokenWrapper Interface
		type TokenWrapper: TokenWrapperInterface<Self::AccountId, Self::AssetId, BalanceOf<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)] //what does this do?
	pub enum Event<T: Config> {
		UpdatedWrappingFeePercent {
			wrapping_fee_percent: BalanceOf<T>,
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Access violation.
		InvalidPermissions,
		// Anchor handler already exists for specified resource Id.
		ResourceIsAlreadyAnchored,
		// Anchor handler doesn't exist for specified resoure Id.
		TokenWrapperHandlerNotFound,
		/// Storage overflowed.
		StorageOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(195_000_000)]
		pub fn execute_wrapping_fee_proposal(
			origin: OriginFor<T>,
			r_id: ResourceId,
			wrapping_fee_percent: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			Self::update_wrapping_fee(r_id, wrapping_fee_percent)
		}
	}
}

impl<T: Config> Pallet<T> {
	fn update_wrapping_fee(r_id: ResourceId, wrapping_fee_percent: BalanceOf<T>) -> DispatchResultWithPostInfo
	{
		T::TokenWrapper::set_wrapping_fee(wrapping_fee_percent); 
		//TODO: have to move set_wrapping_fee to TokenWrapperInterface in token-wrapper pallet...
	}
}