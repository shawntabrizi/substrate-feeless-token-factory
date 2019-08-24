/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use support::{traits::{Currency,  WithdrawReason, ExistenceRequirement}, decl_module, decl_storage, decl_event, ensure,
	Parameter, StorageValue, StorageMap, StorageDoubleMap, dispatch::Result
};
use support::traits::{FindAuthor, Get};
use sr_primitives::ModuleId;
use sr_primitives::traits::{Member, SimpleArithmetic, Zero, StaticLookup, One,
	CheckedAdd, CheckedSub, SignedExtension, DispatchError, MaybeSerializeDebug,
	SaturatedConversion, AccountIdConversion,
};
use sr_primitives::weights::{DispatchInfo, SimpleDispatchInfo};
use sr_primitives::transaction_validity::{TransactionPriority, ValidTransaction};
use system::ensure_signed;
use codec::{Encode, Decode, Codec};

pub trait Trait: system::Trait {

	type Currency: Currency<Self::AccountId>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type TokenBalance: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + MaybeSerializeDebug;

	/// The arithmetic type of asset identifier.
	type TokenId: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + MaybeSerializeDebug;

	type FindAuthor: FindAuthor<Self::AccountId>;

	type TokenFreeTransfers: Parameter + SimpleArithmetic + Default + Copy;

	type FreeTransferPeriod: Get<Self::BlockNumber>;

	type FundTransferFee: Get<BalanceOf<Self>>;
}

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

const MODULE_ID: ModuleId =ModuleId(*b"coinfund");

decl_event!(
	pub enum Event<T> where
		AccountId = <T as system::Trait>::AccountId,
		TokenId = <T as Trait>::TokenId,
		TokenBalance = <T as Trait>::TokenBalance,
		Balance = BalanceOf<T>,
	{
		NewToken(TokenId, AccountId, TokenBalance),
		Transfer(TokenId, AccountId, AccountId, TokenBalance),
		Approval(TokenId, AccountId, AccountId, TokenBalance),
		Deposit(TokenId, AccountId, Balance),
	}
);

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Fungible {
		Count get(count): T::TokenId;

		// ERC 20
		TotalSupply get(total_supply): map T::TokenId => T::TokenBalance;
		Balances get(balance_of): map (T::TokenId, T::AccountId) => T::TokenBalance;
		Allowance get(allowance_of): map (T::TokenId, T::AccountId, T::AccountId) => T::TokenBalance;
		
		// Free Transfers
		FreeTransfers get(free_transfers): map T::TokenId => T::TokenFreeTransfers;
		FreeTransferCount get(free_transfer_count): double_map bool, blake2_128((T::TokenId, T::AccountId)) => T::TokenFreeTransfers;
	}
}

// The module's dispatchable functions.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		/// The time before free transfers are reset
		const FreeTransferPeriod: T::BlockNumber = T::FreeTransferPeriod::get();

		fn on_initialize(n: T::BlockNumber) {
			if n % T::FreeTransferPeriod::get() == Zero::zero() {
				// Reset everyone's transfer count
				<FreeTransferCount<T>>::remove_prefix(&true);
			}
		}

		#[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
		fn create_token(origin, #[compact] total_supply: T::TokenBalance, free_transfers: T::TokenFreeTransfers, deposit: BalanceOf<T>) {
			let sender = ensure_signed(origin)?;

			let id = Self::count();
			let next_id = id.checked_add(&One::one()).ok_or("overflow when adding new token")?;
			let imbalance = T::Currency::withdraw(&sender, deposit, WithdrawReason::Transfer, ExistenceRequirement::KeepAlive)?;

			<Balances<T>>::insert((id, sender.clone()), total_supply);
			<TotalSupply<T>>::insert(id, total_supply);
			<FreeTransfers<T>>::insert(id, free_transfers);
			<Count<T>>::put(next_id);

			T::Currency::resolve_creating(&Self::fund_account_id(id), imbalance);

			Self::deposit_event(RawEvent::NewToken(id, sender.clone(), total_supply));

			Self::deposit_event(RawEvent::Deposit(id, sender, deposit));
		}

		#[weight = SimpleDispatchInfo::FixedNormal(0)]
		fn try_free_transfer(origin,
			#[compact] id: T::TokenId,
			to: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::TokenBalance
		) {
			let sender = ensure_signed(origin)?;
			let to = T::Lookup::lookup(to)?;

			let free_transfer_limit = Self::free_transfers(id);
			let new_free_transfer_count = Self::free_transfer_count(&true, &(id, sender.clone()))
				.checked_add(&One::one()).ok_or("overflow when counting new transfer")?;

			ensure!(free_transfer_limit < new_free_transfer_count, "no more free transfers available");

			// Burn fees from funds
			let fund_account = Self::fund_account_id(id);
			let fund_fee = T::FundTransferFee::get();
			let _ = T::Currency::withdraw(&fund_account, fund_fee, WithdrawReason::Transfer, ExistenceRequirement::AllowDeath)?;

			Self::make_transfer(id, sender.clone(), to, amount)?;

			<FreeTransferCount<T>>::insert(&true, &(id, sender), &new_free_transfer_count);
		}

		fn transfer(origin,
			#[compact] id: T::TokenId,
			to: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::TokenBalance
		) {
			let sender = ensure_signed(origin)?;
			let to = T::Lookup::lookup(to)?;

			Self::make_transfer(id, sender, to, amount)?;
		}

		fn approve(origin,
			#[compact] id: T::TokenId,
			spender: <T::Lookup as StaticLookup>::Source,
			#[compact] value: T::TokenBalance
		) {
			let sender = ensure_signed(origin)?;
			let spender = T::Lookup::lookup(spender)?;

			<Allowance<T>>::insert((id, sender.clone(), spender.clone()), value);
			
			Self::deposit_event(RawEvent::Approval(id, sender, spender, value));
		}

		fn transfer_from(origin,
			#[compact] id: T::TokenId,
			from: T::AccountId,
			to: T::AccountId,
			#[compact] value: T::TokenBalance
		) {
			let sender = ensure_signed(origin)?;
			let allowance = Self::allowance_of((id, from.clone(), sender.clone()));

			let updated_allowance = allowance.checked_sub(&value).ok_or("underflow in calculating allowance")?;

			Self::make_transfer(id, from.clone(), to.clone(), value)?;

			<Allowance<T>>::insert((id, from, sender), updated_allowance);
		}

		fn deposit(origin, #[compact] token_id: T::TokenId, #[compact] value: BalanceOf<T>) {
			let who = ensure_signed(origin)?;
			ensure!(Self::count() > token_id, "Non-existent token");
			T::Currency::transfer(&who, &Self::fund_account_id(token_id), value)?;

			Self::deposit_event(RawEvent::Deposit(token_id, who, value));
		}
	}
}

impl<T: Trait> Module<T> {
	pub fn fund_account_id(index: T::TokenId) -> T::AccountId {
		MODULE_ID.into_sub_account(index)
	}

	fn make_transfer(id: T::TokenId, from: T::AccountId, to: T::AccountId, amount: T::TokenBalance) -> Result {
		ensure!(!amount.is_zero(), "transfer amount should be non-zero");
		
		let from_balance = Self::balance_of((id, from.clone()));
		ensure!(from_balance >= amount.clone(), "origin account balance must be greater than or equal to the transfer amount");

		<Balances<T>>::insert((id, from.clone()), from_balance - amount.clone());
		<Balances<T>>::mutate((id, to.clone()), |balance| *balance += amount.clone());

		Self::deposit_event(RawEvent::Transfer(id, from, to, amount));

		Ok(())
	}
}

/// Allow payment of fees using the token being transferred.
#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct TakeTokenFees<T: Trait> {
	id: T::TokenId,
	value: T::TokenBalance,
}

#[cfg(feature = "std")]
impl<T: Trait> rstd::fmt::Debug for TakeTokenFees<T>
{
	fn fmt(&self, f: &mut rstd::fmt::Formatter) -> rstd::fmt::Result {
		// TODO: Fix this to actually show value
		write!(f, "TokenFee ( id: ?, fee: ? )")
	}
}

impl<T: Trait> SignedExtension for TakeTokenFees<T> {
	type AccountId = T::AccountId;
	type Call = T::Call;
	type AdditionalSigned = ();
	type Pre = ();
	fn additional_signed(&self) -> rstd::result::Result<(), &'static str> { Ok(()) }

	fn validate(
		&self,
		who: &Self::AccountId,
		_call: &Self::Call,
		_info: DispatchInfo,
		_len: usize,
	) -> rstd::result::Result<ValidTransaction, DispatchError> {
		
		let id = self.id;
		let fee = self.value;

		// TODO: Actually look up the block author and transfer to them
		// let digest = <system::Module<T>>::digest();
		// let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
		// // TODO: FIX
		// if let Some(author) = T::FindAuthor::find_author(pre_runtime_digests) {
		// 	<Module<T>>::make_transfer(id, who, author, fee);
		// } else {
		// 	Default::default()
		// }

		let result = <Module<T>>::make_transfer(id, who.clone(), Default::default(), fee);


		let mut r = ValidTransaction::default();
		if result.is_ok() {
			// TODO: Calculate priority based on percentage of total supply
			r.priority = fee.saturated_into::<TransactionPriority>();
		}
		Ok(r)
	}
}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok, assert_err, parameter_types};
	use sr_primitives::{
		traits::{BlakeTwo256, IdentityLookup},
		testing::Header};
	use sr_primitives::weights::Weight;
	use sr_primitives::Perbill;

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type TransactionPayment = ();
		type TransferPayment = ();
		type DustRemoval = ();
		type Event = ();
		type ExistentialDeposit = ();
		type TransferFee = ();
		type CreationFee = ();
		type TransactionBaseFee = ();
		type TransactionByteFee = ();
		type WeightToFee = ();
	}

	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type WeightMultiplierUpdate = ();
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
	}
	impl Trait for Test {
		type Event = ();
		type TokenBalance = u64;
		type TokenId = u32;
		type Currency = Balances;
		type TokenFreeTransfers = u32;
		type FindAuthor = ();
	}
	type FungibleModule = Module<Test>;
	type Balances = balances::Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let _ = balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 100),
				(2, 500),
			],
			vesting: vec![],
		}.assimilate_storage(&mut t).unwrap();

		t.into()
	}

	#[test]
	fn it_deposits_more_currency_to_token() {
		with_externalities(&mut new_test_ext(), || {
			// asserting that the stored value is equal to what we stored
		 	let origin = Origin::signed(1);
			<Count<Test>>::put(1);
			let token_id = 0;
			let value = 40;

			assert_ok!(FungibleModule::deposit(origin, token_id, value));

			assert_eq!(Balances::free_balance(&1), 60);
		});
	}

	#[test]
	fn it_does_not_deposit_if_not_enough_balance() {
		with_externalities(&mut new_test_ext(), || {
			// asserting that the stored value is equal to what we stored
			let origin = Origin::signed(1);
			<Count<Test>>::put(1);
			let token_id = 0;
			let value = 101;

			assert_err!(FungibleModule::deposit(origin, token_id, value), "balance too low to send value");

			assert_eq!(Balances::free_balance(&1), 100);
		});
	}
}
