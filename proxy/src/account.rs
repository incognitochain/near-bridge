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
