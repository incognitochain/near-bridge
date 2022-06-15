mod account;
mod errors;
mod token_receiver;
mod utils;
mod w_near;

use account::Account;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, near_bindgen, serde_json, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise, ext_contract, PromiseOrValue, PromiseResult,
};
use utils::WRAP_NEAR_ACCOUNT;

use crate::errors::*;
use crate::utils::{BRIDGE_CONTRACT, GAS_FOR_WNEAR, GAS_FOR_RESOLVE_WNEAR};
use crate::w_near::ext_wnear;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum DappRequest {
    WrapNear {},
    DepositRefFinance {},
    SwapRefFinance {},
    WithdrawRefFinace {},
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Account,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Proxy {
    accounts: LookupMap<AccountId, Account>,
}

#[ext_contract(ext_self)]
pub trait ProxyContract {
    fn callback_wnear(&mut self);
}

#[near_bindgen]
impl Proxy {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");

        let mut this = Self {
            accounts: LookupMap::new(StorageKey::Account),
        };

        this
    }

    #[payable]
    pub fn deposit_near(&mut self, account_id: AccountId, wrap: bool) -> Promise {
        let amount = env::attached_deposit();
        assert!(amount > 0, "Requires positive attached deposit");

        // ! Storage deposit first to all registed contracts.

        ext_wnear::near_deposit(account_id, amount, GAS_FOR_WNEAR).
        then(ext_self::callback_wnear(
            env::current_account_id().clone(),
            0,
            GAS_FOR_RESOLVE_WNEAR,
        )).into()
    }

    pub fn call_dapp(&mut self, msg: String) {
        let sender_id = env::predecessor_account_id();
        assert_eq!(sender_id.to_string(), BRIDGE_CONTRACT);

        let message = serde_json::from_str::<DappRequest>(&msg).expect(ERR28_WRONG_MSG_FORMAT);
        match message {
            DappRequest::WrapNear {} => {}
            DappRequest::DepositRefFinance {} => {}
            DappRequest::SwapRefFinance {} => {}
            DappRequest::WithdrawRefFinace {} => {}
        }
    }

    pub fn withdraw(&mut self, token_id: String, amount: u128, receiver_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        assert_eq!(sender_id.to_string(), BRIDGE_CONTRACT);

        // TODO
    }

    pub fn callback_wnear(&mut self) -> PromiseOrValue<U128> {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        // handle the result from the second cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("{}", WNEAR_CALLBACK_FAILED),
            PromiseResult::Successful(_result) => {
                PromiseOrValue::Value(U128(0))
            }
        }
    }
}

/// Internal methods implementation.
impl Proxy {
    pub(crate) fn internal_deposit_token(
        &mut self,
        account_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) {
        let mut account = self.internal_unwrap_account(account_id);
        account.deposit_token(token_id, amount);
        self.internal_save_account(&account_id, account);
    }

    pub(crate) fn internal_deposit_near(&mut self, account_id: &AccountId, amount: Balance) {
        let mut account = self.internal_unwrap_account(account_id);
        account.deposit_near(amount);
        self.internal_save_account(&account_id, account);
    }

    pub fn internal_unwrap_account(&self, account_id: &AccountId) -> Account {
        let account = self.accounts.get(account_id);
        match account {
            Some(account) => account,
            None => Account::new(),
        }
    }

    pub(crate) fn internal_save_account(&mut self, account_id: &AccountId, account: Account) {
        // TODO: assert storage
        self.accounts.insert(&account_id, &account.into());
    }
}
