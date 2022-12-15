use super::*;
use frame_support::pallet_prelude::*;
use orml_tokens::AccountData;
use orml_traits::{
	currency::{OnDeposit, OnSlash, OnTransfer},
	Happened,
};
use sp_std::{cell::RefCell, marker, prelude::*, thread};

thread_local! {
	pub static CREATED: RefCell<Vec<(AccountId, CurrencyId)>> = RefCell::new(vec![]);
	pub static KILLED: RefCell<Vec<(AccountId, CurrencyId)>> = RefCell::new(vec![]);
}

pub struct TrackCreatedAccounts<T>(marker::PhantomData<T>);
impl<T: orml_tokens::Config> TrackCreatedAccounts<T>
where
	T::AccountId: From<AccountId32> + Into<AccountId32>,
	T::CurrencyId: From<u32> + Into<u32>,
{
	pub fn accounts() -> Vec<(T::AccountId, T::CurrencyId)> {
		CREATED
			.with(|accounts| accounts.borrow().clone())
			.iter()
			.map(|account| (account.0.clone().into(), account.1.clone().into()))
			.collect()
	}

	pub fn reset() {
		CREATED.with(|accounts| {
			accounts.replace(vec![]);
		});
	}
}
impl<T: orml_tokens::Config> Happened<(T::AccountId, T::CurrencyId)> for TrackCreatedAccounts<T>
where
	T::AccountId: From<AccountId32> + Into<AccountId32>,
	T::CurrencyId: From<u32> + Into<u32>,
{
	fn happened((who, currency): &(T::AccountId, T::CurrencyId)) {
		CREATED.with(|accounts| {
			accounts.borrow_mut().push((who.clone().into(), (*currency).into()));
		});
	}
}

pub struct TrackKilledAccounts<T>(marker::PhantomData<T>);
impl<T: orml_tokens::Config> TrackKilledAccounts<T>
where
	T::AccountId: From<AccountId32> + Into<AccountId32>,
	T::CurrencyId: From<u32> + Into<u32>,
{
	pub fn accounts() -> Vec<(T::AccountId, T::CurrencyId)> {
		KILLED
			.with(|accounts| accounts.borrow().clone())
			.iter()
			.map(|account| (account.0.clone().into(), account.1.clone().into()))
			.collect()
	}

	pub fn reset() {
		KILLED.with(|accounts| {
			accounts.replace(vec![]);
		});
	}
}
impl<T: orml_tokens::Config> Happened<(T::AccountId, T::CurrencyId)> for TrackKilledAccounts<T>
where
	T::AccountId: From<AccountId32> + Into<AccountId32>,
	T::CurrencyId: From<u32> + Into<u32>,
{
	fn happened((who, currency): &(T::AccountId, T::CurrencyId)) {
		KILLED.with(|accounts| {
			accounts.borrow_mut().push((who.clone().into(), (*currency).into()));
		});
	}
}

thread_local! {
	pub static ON_SLASH_CALLS: RefCell<u32> = RefCell::new(0);
	pub static ON_DEPOSIT_PREHOOK_CALLS: RefCell<u32> = RefCell::new(0);
	pub static ON_DEPOSIT_POSTHOOK_CALLS: RefCell<u32> = RefCell::new(0);
	pub static ON_TRANSFER_PREHOOK_CALLS: RefCell<u32> = RefCell::new(0);
	pub static ON_TRANSFER_POSTHOOK_CALLS: RefCell<u32> = RefCell::new(0);
}

pub struct OnSlashHook<T>(marker::PhantomData<T>);
impl<T: orml_tokens::Config> OnSlash<T::AccountId, T::CurrencyId, T::Balance> for OnSlashHook<T> {
	fn on_slash(_currency_id: T::CurrencyId, _account_id: &T::AccountId, _amount: T::Balance) {
		ON_SLASH_CALLS.with(|cell| *cell.borrow_mut() += 1);
	}
}
impl<T: orml_tokens::Config> OnSlashHook<T> {
	pub fn calls() -> u32 {
		ON_SLASH_CALLS.with(|accounts| *accounts.borrow())
	}
}

pub struct PreDeposit<T>(marker::PhantomData<T>);
impl<T: orml_tokens::Config> OnDeposit<T::AccountId, T::CurrencyId, T::Balance> for PreDeposit<T> {
	fn on_deposit(
		_currency_id: T::CurrencyId,
		_account_id: &T::AccountId,
		_amount: T::Balance,
	) -> DispatchResult {
		ON_DEPOSIT_PREHOOK_CALLS.with(|cell| *cell.borrow_mut() += 1);
		Ok(())
	}
}
impl<T: orml_tokens::Config> PreDeposit<T> {
	pub fn calls() -> u32 {
		ON_DEPOSIT_PREHOOK_CALLS.with(|accounts| accounts.borrow().clone())
	}
}

pub struct PostDeposit<T>(marker::PhantomData<T>);
impl<T: orml_tokens::Config> OnDeposit<T::AccountId, T::CurrencyId, T::Balance> for PostDeposit<T> {
	fn on_deposit(
		currency_id: T::CurrencyId,
		account_id: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		ON_DEPOSIT_POSTHOOK_CALLS.with(|cell| *cell.borrow_mut() += 1);
		let account_balance: AccountData<T::Balance> = orml_tokens::Pallet::<T>::accounts::<
			T::AccountId,
			T::CurrencyId,
		>(account_id.clone(), currency_id);
		assert!(
			account_balance.free.ge(&amount),
			"Posthook must run after the account balance is updated."
		);
		Ok(())
	}
}
impl<T: orml_tokens::Config> PostDeposit<T> {
	pub fn calls() -> u32 {
		ON_DEPOSIT_POSTHOOK_CALLS.with(|accounts| accounts.borrow().clone())
	}
}

pub struct PreTransfer<T>(marker::PhantomData<T>);
impl<T: orml_tokens::Config> OnTransfer<T::AccountId, T::CurrencyId, T::Balance>
	for PreTransfer<T>
{
	fn on_transfer(
		_currency_id: T::CurrencyId,
		_from: &T::AccountId,
		_to: &T::AccountId,
		_amount: T::Balance,
	) -> DispatchResult {
		ON_TRANSFER_PREHOOK_CALLS.with(|cell| *cell.borrow_mut() += 1);
		Ok(())
	}
}
impl<T: orml_tokens::Config> PreTransfer<T> {
	pub fn calls() -> u32 {
		ON_TRANSFER_PREHOOK_CALLS.with(|accounts| accounts.borrow().clone())
	}
}

pub struct PostTransfer<T>(marker::PhantomData<T>);
impl<T: orml_tokens::Config> OnTransfer<T::AccountId, T::CurrencyId, T::Balance>
	for PostTransfer<T>
{
	fn on_transfer(
		currency_id: T::CurrencyId,
		_from: &T::AccountId,
		to: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		ON_TRANSFER_POSTHOOK_CALLS.with(|cell| *cell.borrow_mut() += 1);
		let account_balance: AccountData<T::Balance> = orml_tokens::Pallet::<T>::accounts::<
			T::AccountId,
			T::CurrencyId,
		>(to.clone(), currency_id);
		assert!(
			account_balance.free.ge(&amount),
			"Posthook must run after the account balance is updated."
		);
		Ok(())
	}
}
impl<T: orml_tokens::Config> PostTransfer<T> {
	pub fn calls() -> u32 {
		ON_TRANSFER_POSTHOOK_CALLS.with(|accounts| accounts.borrow().clone())
	}
}
