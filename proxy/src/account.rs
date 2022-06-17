use crate::*;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{AccountId, Balance};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    /// Native NEAR amount sent to the proxy.
    pub near_amount: Balance,
    /// Amounts of various tokens deposited to this account.
    pub tokens: LookupMap<AccountId, Balance>,
}

impl Account {
    pub fn new() -> Self {
        Account {
            near_amount: 0,
            tokens: LookupMap::new(StorageKey::Account {}),
        }
    }

    pub(crate) fn get_balance_token(&self, token_id: &AccountId) -> Balance {
        self.tokens.get(token_id).unwrap_or(0)
    }

    /// Deposit amount to the balance of given token.
    pub(crate) fn deposit_token(&mut self, token: &AccountId, amount: Balance) {
        if amount > 0 {
            if let Some(x) = self.tokens.get(token) {
                self.tokens.insert(token, &(amount + x));
            } else {
                self.tokens.insert(token, &amount);
            }
        }
    }

    /// Deposit amount to the balance of given token.
    pub(crate) fn withdraw_token(&mut self, token: &AccountId, amount: Balance) {
        if amount > 0 {
            if let Some(x) = self.tokens.get(token) {
                if x >= amount {
                    self.tokens.insert(token, &(x - amount));
                } else {
                    panic!("Insufficient balance");
                }
            } else {
                panic!("Insufficient balance");
            }
        }
    }

    pub(crate) fn deposit_near(&mut self, amount: Balance) {
        self.near_amount += amount;
    }
}