use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, env, PromiseOrValue, Gas};
use near_sdk::AccountId;
use near_sdk::json_types::U128;

use crate::utils::{WRAP_NEAR_ACCOUNT, REF_FINANCE_ACCOUNT};

use crate::errors::*;
use crate::*;

#[near_bindgen]
impl FungibleTokenReceiver for Proxy {
    /// Callback on receiving tokens by this contract.
    /// `msg` format is either "" for deposit or `TokenReceiverMessage`.
    #[allow(unreachable_code)]
    fn ft_on_transfer(
        &mut self,
        _sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_in = env::predecessor_account_id();
        if msg.is_empty() {
            panic!("{}", INVALID_MESSAGE)
        }
        // shield request
        let message =
            serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR28_WRONG_MSG_FORMAT);
        match message {
            TokenReceiverMessage::DappRequest { dapp_account, payload } => {
                let amount = amount.0;
                if dapp_account == REF_FINANCE_ACCOUNT {
                    PromiseOrValue::Value(U128(0))
                } else {
                    panic!("{}", INVALID_MESSAGE)
                }
            }
        }
    }
}
