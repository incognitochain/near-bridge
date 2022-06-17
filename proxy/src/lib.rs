mod account;
mod errors;
mod token_receiver;
mod utils;
mod w_near;

use std::convert::TryInto;

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
use crate::utils::{BRIDGE_CONTRACT, GAS_FOR_WNEAR, GAS_FOR_RESOLVE_WNEAR, GAS_FOR_DEPOSIT, GAS_FOR_RESOLVE_DEPOSIT};
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
    fn callback_wnear(&mut self, account_id: AccountId, amount: U128);
    fn callback_withdraw(&mut self, account_id: AccountId, token_id: AccountId, amount: U128);
}

#[ext_contract(ext_ft)]
pub trait FtContract {
    fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>, msg: String);
}

#[ext_contract(ext_bridge)]
pub trait BridgeContract {
    fn deposit(&mut self, incognito_address: String,
    );
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]

pub struct Deposit {
    incognito_address: String
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
    pub fn deposit_near(&mut self, account_id: AccountId, _wrap: bool) -> Promise {
        let amount = env::attached_deposit();
        assert!(amount > 0, "Requires positive attached deposit");

        // ! Storage deposit first to all registed contracts.

        ext_wnear::near_deposit(account_id.clone(), amount, GAS_FOR_WNEAR).
        then(ext_self::callback_wnear(
            account_id.clone(),
            U128(amount),
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

    pub fn withdraw(&mut self, token_id: String, amount: u128, account_id: AccountId, incognito_address: String) -> Promise {
        let sender_id = env::predecessor_account_id();
        assert_eq!(sender_id.to_string(), BRIDGE_CONTRACT);

        let mut withdraw_token = WRAP_NEAR_ACCOUNT.to_string();
        if token_id != "" { // not withdraw NEAR
            withdraw_token = token_id.clone();
        }
        let withdraw_token_id: AccountId = withdraw_token.clone().try_into().unwrap();

        let mut withdraw_amount = amount;
        if amount == 0 {
            withdraw_amount = self.internal_get_balance_token(&account_id, &withdraw_token_id);
        }

        let bridge_id: AccountId = BRIDGE_CONTRACT.to_string().try_into().unwrap();

        self.internal_withdraw_token(&account_id, &withdraw_token_id, withdraw_amount);

        if token_id != "" { // not withdraw NEAR
            let obj = Deposit {
                incognito_address: incognito_address.clone(),
            };
            let msg = serde_json::to_string(&obj).unwrap();
            ext_ft::ft_transfer_call(
                bridge_id,
                U128(withdraw_amount),
                None,
                msg,
                withdraw_token_id.clone(),
                1,
                GAS_FOR_DEPOSIT,
            ).then(ext_self::callback_withdraw(
                account_id.clone(),
                withdraw_token_id.clone(),
                U128(withdraw_amount),
                env::current_account_id().clone(),
                0,
                GAS_FOR_RESOLVE_DEPOSIT,
            )).into()
        } else {
            ext_bridge::deposit(
                incognito_address, 
                bridge_id,
                withdraw_amount,
                GAS_FOR_DEPOSIT,
            ).into()
        }
    }

    pub fn callback_wnear(&mut self, account_id: AccountId, amount: U128) -> PromiseOrValue<U128> {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        let wnear_id: AccountId = WRAP_NEAR_ACCOUNT.to_string().try_into().unwrap();

        // handle the result from the second cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("{}", WNEAR_CALLBACK_FAILED),
            PromiseResult::Successful(_result) => {
                self.internal_deposit_token(&account_id, &wnear_id, amount.into());

                PromiseOrValue::Value(U128(0))
            }
        }
    }

    pub fn callback_withdraw(&mut self, account_id: AccountId, token_id: AccountId, amount: U128) {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        // handle the result from the second cross contract call this method is a callback for
       match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                self.internal_deposit_token(&account_id, &token_id, amount.into());
            }
            PromiseResult::Successful(result) => {}
        };
    }
}

/// Internal methods implementation.
impl Proxy {
    pub(crate) fn internal_get_balance_token(&self, account_id: &AccountId, token_id: &AccountId) -> Balance {
        let account = self.internal_unwrap_account(account_id);
        account.get_balance_token(token_id)
    }

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

    pub(crate) fn internal_withdraw_token(
        &mut self,
        account_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) {
        let mut account = self.internal_unwrap_account(account_id);
        account.withdraw_token(token_id, amount);
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
