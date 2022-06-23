mod account;
mod errors;
mod ref_finance;
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
    env, ext_contract, near_bindgen, serde_json, AccountId, Balance, BorshStorageKey,
    PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
};
use ref_finance::SwapAction;
use utils::WRAP_NEAR_ACCOUNT;

use crate::errors::*;
use crate::ref_finance::ext_ref_finance;
use crate::utils::{
    BRIDGE_CONTRACT, GAS_FOR_DEPOSIT, GAS_FOR_RESOLVE_DEPOSIT, GAS_FOR_RESOLVE_SWAP_REF_FINACE,
    GAS_FOR_RESOLVE_WITHDRAW_REF_FINACE, GAS_FOR_RESOLVE_WNEAR, GAS_FOR_SWAP_REF_FINANCE,
    GAS_FOR_WITHDRAW_REF_FINANCE, GAS_FOR_WNEAR, REF_FINANCE_ACCOUNT,
};
use crate::w_near::ext_wnear;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum DappRequest {
    SwapRefFinance {
        action: SwapAction,
        account_id: AccountId,
    },
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Account,
    Token,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Proxy {
    accounts: LookupMap<AccountId, Account>,
}

#[ext_contract(this_contract)]
pub trait Callbacks {
    fn callback_wnear(&mut self, account_id: AccountId, amount: U128);
    fn callback_swap_ref_finance(&mut self, action: SwapAction, verifier: AccountId);
    fn callback_withdraw_ref_finace(
        &mut self,
        account_id: AccountId,
        token: AccountId,
        amount: U128,
    );
    fn callback_withdraw(
        &mut self,
        account_id: AccountId,
        token_id: AccountId,
        amount: U128,
        unwrap: bool,
    );
}

#[ext_contract(ext_ft)]
pub trait FtContract {
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    );
}

#[ext_contract(ext_bridge)]
pub trait BridgeContract {
    fn deposit(&mut self, incognito_address: String);
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]

pub struct Deposit {
    incognito_address: String,
}

#[near_bindgen]
impl Proxy {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");

        let this = Self {
            accounts: LookupMap::new(StorageKey::Account),
        };

        this
    }

    #[payable]
    pub fn deposit_near(&mut self, account_id: AccountId, wrap: bool) -> Promise {
        let amount = env::attached_deposit();
        assert!(amount > 0, "Requires positive attached deposit");

        // ! Storage deposit first to all registed contracts.

        let wnear_id: AccountId = WRAP_NEAR_ACCOUNT.to_string().try_into().unwrap();

        let near_deposit_ps = ext_wnear::ext(wnear_id)
            .with_static_gas(GAS_FOR_WNEAR)
            .with_attached_deposit(amount)
            .near_deposit();

        near_deposit_ps.then(
            Self::ext(env::current_account_id().clone())
            .with_static_gas(GAS_FOR_RESOLVE_WNEAR)
            .callback_wnear(
                account_id.clone(),
                U128(amount),
            )
        ).into()
    }

    pub fn call_dapp(&mut self, msg: String) -> Promise {
        let sender_id = env::predecessor_account_id();
        assert_eq!(sender_id.to_string(), BRIDGE_CONTRACT);

        let message = serde_json::from_str::<DappRequest>(&msg).expect(ERR28_WRONG_MSG_FORMAT);
        match message {
            DappRequest::SwapRefFinance {
                action:
                    SwapAction {
                        pool_id,
                        token_in,
                        amount_in,
                        token_out,
                        min_amount_out,
                    },
                account_id,
            } => {
                self.internal_withdraw_token(
                    &account_id,
                    &token_in.clone(),
                    amount_in.clone().unwrap().into(),
                );

                let ref_finance_id: AccountId = REF_FINANCE_ACCOUNT.to_string().try_into().unwrap();

                let ft_transfer_call_ps = ext_ft::ext(token_in.clone())
                    .with_static_gas(GAS_FOR_DEPOSIT)
                    .with_attached_deposit(1)
                    .ft_transfer_call(
                        ref_finance_id.clone(),
                        amount_in.clone().unwrap(),
                        None,
                        "".to_string(),
                    );

                let swap_ps = ext_ref_finance::ext(ref_finance_id.clone())
                    .with_static_gas(GAS_FOR_SWAP_REF_FINANCE)
                    .with_attached_deposit(1)
                    .swap(
                        vec![SwapAction {
                            pool_id,
                            token_in: token_in.clone(),
                            amount_in,
                            token_out: token_out.clone(),
                            min_amount_out,
                        }],
                        Some(env::current_account_id()),
                );

                ft_transfer_call_ps
                .then(swap_ps)
                .then(Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_SWAP_REF_FINACE)
                    .callback_swap_ref_finance(
                        SwapAction {
                            pool_id,
                            token_in: token_in.clone(),
                            amount_in,
                            token_out: token_out.clone(),
                            min_amount_out,
                        },
                        account_id,
                    )).into()
            }
        }
    }

    pub fn withdraw(
        &mut self,
        token_id: String,
        amount: u128,
        account_id: AccountId,
        incognito_address: String,
    ) -> Promise {
        let sender_id = env::predecessor_account_id();
        assert_eq!(sender_id.to_string(), BRIDGE_CONTRACT);

        let mut withdraw_token = WRAP_NEAR_ACCOUNT.to_string();
        if token_id != "" {
            // not withdraw NEAR
            withdraw_token = token_id.clone();
        }
        let withdraw_token_id: AccountId = withdraw_token.clone().try_into().unwrap();

        let mut withdraw_amount = amount;
        if amount == 0 {
            withdraw_amount = self.internal_get_balance_token(&account_id, &withdraw_token_id);
        }

        let bridge_id: AccountId = BRIDGE_CONTRACT.to_string().try_into().unwrap();

        self.internal_withdraw_token(&account_id, &withdraw_token_id, withdraw_amount);

        if token_id != "" {
            // not withdraw NEAR
            let obj = Deposit {
                incognito_address: incognito_address.clone(),
            };
            let msg = serde_json::to_string(&obj).unwrap();

            let ft_transfer_call_ps = ext_ft::ext(withdraw_token_id.clone())
                .with_static_gas(GAS_FOR_DEPOSIT)
                .with_attached_deposit(1)
                .ft_transfer_call(
                    bridge_id,
                    U128(withdraw_amount),
                    None,
                    msg,
                );

            ft_transfer_call_ps
            .then(Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_RESOLVE_DEPOSIT)
                .callback_withdraw(
                    account_id.clone(),
                    withdraw_token_id.clone(),
                    U128(withdraw_amount),
                    false,
                )).into()
        } else {
            let near_withdraw_ps = ext_wnear::ext(WRAP_NEAR_ACCOUNT.to_string().try_into().unwrap())
                .with_static_gas(GAS_FOR_WNEAR)
                .with_attached_deposit(1)
                .near_withdraw(
                    U128(withdraw_amount - 1),
                );

            let deposit_ps = ext_bridge::ext(WRAP_NEAR_ACCOUNT.to_string().try_into().unwrap())
                .with_static_gas(GAS_FOR_DEPOSIT)
                .with_attached_deposit(withdraw_amount)
                .deposit(
                    incognito_address,
                );

            near_withdraw_ps
            .then(deposit_ps)
            .then(Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_RESOLVE_DEPOSIT)
                .callback_withdraw(
                    account_id.clone(),
                    withdraw_token_id.clone(),
                    U128(withdraw_amount),
                    true,
                )).into()
        }
    }

    pub fn callback_wnear(&mut self, account_id: AccountId, amount: U128) -> PromiseOrValue<U128> {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        let wnear_id: AccountId = WRAP_NEAR_ACCOUNT.to_string().try_into().unwrap();

        // handle the result from the first cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("{}", WNEAR_CALLBACK_FAILED),
            PromiseResult::Successful(_result) => {
                self.internal_deposit_token(&account_id, &wnear_id, amount.into());

                PromiseOrValue::Value(U128(0))
            }
        }
    }

    pub fn callback_swap_ref_finance(
        &mut self,
        action: SwapAction,
        account_id: AccountId,
    ) -> PromiseOrValue<U128> {
        assert_eq!(env::promise_results_count(), 2, "This is a callback method");

        let deposit_success: bool = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => false,
            PromiseResult::Successful(_result) => true,
        };

        let swap_result: Option<U128> = match env::promise_result(1) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => None,
            PromiseResult::Successful(result) => near_sdk::serde_json::from_slice::<U128>(&result)
                .unwrap()
                .into(),
        };

        let ref_finance_id: AccountId = REF_FINANCE_ACCOUNT.to_string().try_into().unwrap();

        if !deposit_success || swap_result.is_none() {
            if deposit_success {
                let withdraw_ps = ext_ref_finance::ext(ref_finance_id)
                    .with_static_gas(GAS_FOR_WITHDRAW_REF_FINANCE)
                    .with_attached_deposit(1)
                    .withdraw(
                        action.token_in.clone(),
                        action.amount_in.unwrap(),
                        None,
                    );

                withdraw_ps.then(Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_RESOLVE_WITHDRAW_REF_FINACE)
                .callback_withdraw_ref_finace(
                    account_id,
                    action.token_in.clone(),
                    action.amount_in.unwrap(),
                )).into()
            } else {
                PromiseOrValue::Value(U128(0))
            }
        } else {
            let withdraw_ps = ext_ref_finance::ext(ref_finance_id)
                .with_static_gas(GAS_FOR_WITHDRAW_REF_FINANCE)
                .with_attached_deposit(1)
                .withdraw(
                    action.token_out.clone(),
                    swap_result.unwrap(),
                    None,
                );

            withdraw_ps.then(Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_RESOLVE_WITHDRAW_REF_FINACE)
                .callback_withdraw_ref_finace(
                    account_id,
                    action.token_out.clone(),
                    swap_result.unwrap(),
                )).into()
        }
    }

    pub fn callback_withdraw_ref_finace(
        &mut self,
        account_id: AccountId,
        token_id: AccountId,
        amount: U128,
    ) {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("{}", WITHDRAW_REF_FINANCE_CALLBACK_FAILED),
            PromiseResult::Successful(_result) => {
                self.internal_deposit_token(&account_id, &token_id, amount.into());
            }
        };
    }

    pub fn callback_withdraw(
        &mut self,
        account_id: AccountId,
        token_id: AccountId,
        amount: U128,
        unwrap: bool,
    ) {
        let mut num_promise: u64 = 1;
        if unwrap {
            num_promise = 2;
        }
        assert_eq!(
            env::promise_results_count(),
            num_promise,
            "This is a callback method"
        );

        // handle the result from the last cross contract call this method is a callback for
        match env::promise_result(num_promise - 1) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                env::log_str(format!("WithdrawFailed {} {} {}", account_id, token_id, u128::from(amount)).as_str());
                self.internal_deposit_token(&account_id, &token_id, amount.into());
                return;
            }
            PromiseResult::Successful(_result) => {
                env::log_str(format!("WithdrawSuccess {} {} {}", account_id, token_id, u128::from(amount)).as_str());
            }
        };
    }

    /// getters
    
    pub fn get_balance_token(&self, account_id: &AccountId, token_id: &AccountId) -> Balance {
        return self.internal_get_balance_token(account_id, token_id);
    }
}

/// Internal methods implementation.
impl Proxy {
    pub(crate) fn internal_get_balance_token(
        &self,
        account_id: &AccountId,
        token_id: &AccountId,
    ) -> Balance {
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
