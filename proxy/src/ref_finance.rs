use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;

use crate::*;

/// Single swap action.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapAction {
    /// Pool which should be used for swapping.
    pub pool_id: u64,
    /// Token to swap from.
    pub token_in: AccountId,
    /// Amount to exchange.
    /// If amount_in is None, it will take amount_out from previous step.
    /// Will fail if amount_in is None on the first step.
    pub amount_in: Option<U128>,
    /// Token to swap into.
    pub token_out: AccountId,
    /// Required minimum amount of token_out.
    pub min_amount_out: U128,
}

#[ext_contract(ext_ref_finance)]
pub(crate) trait RefFinance {
    fn swap(&mut self, actions: Vec<SwapAction>, referral_id: Option<AccountId>) -> U128;
    fn withdraw(&mut self, token_id: AccountId, amount: U128, unregister: Option<bool>);
}