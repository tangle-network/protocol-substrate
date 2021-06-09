#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get};
use frame_system::ensure_signed;
use sp_std::prelude::*;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub trait InstanceHasher {
	fn hash(data: &[u8]) -> Vec<u8>;
}

/// Configure the pallet by specifying the parameters and types on which it
/// depends.
pub trait Config<I: Instance = DefaultInstance>: frame_system::Config {
	/// Because this pallet emits events, it depends on the runtime's definition
	/// of an event.
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Config>::Event>;
	type Hasher: InstanceHasher;
}

decl_storage! {
	trait Store for Module<T: Config<I>, I: Instance=DefaultInstance> as HasherModule {
		Something get(fn something): Option<u32>;
	}
}

decl_event!(
	pub enum Event<T, I: Instance = DefaultInstance>
	where
		AccountId = <T as frame_system::Config>::AccountId,
	{
		/// Event documentation should end with an array that provides
		/// descriptive names for event parameters. [something, who]
		SomethingStored(u32, AccountId),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Config<I>, I: Instance> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}
}

// Dispatchable functions allows users to interact with the pallet and invoke
// state changes. These functions materialize as "extrinsics", which are often
// compared to transactions. Dispatchable functions must be annotated with a
// weight and must return a DispatchResult.
decl_module! {
	pub struct Module<T: Config<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T, I>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn hash(origin, data: Vec<u8>) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			let _ = T::Hasher::hash(&data);
			Ok(())
		}
	}
}
