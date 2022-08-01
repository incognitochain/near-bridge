use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;
use near_sdk::{env, serde_json, PromiseOrValue};

use crate::errors::*;
use crate::*;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    Deposit { account_id: AccountId },
    Execute {
        call_data: String,
        withdraw_address: String,
        incognito_address: String
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for Proxy {
    /// Callback on receiving tokens by this contract.
    /// `msg` format is either "" for deposit or `TokenReceiverMessage`.
    #[allow(unreachable_code)]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_in = env::predecessor_account_id();
        let amount = amount.0;
        if msg.is_empty() {
            self.internal_deposit_token(&sender_id, &token_in, amount.into());
            PromiseOrValue::Value(U128(0))
        } else {
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::Deposit { account_id } => {
                    self.internal_deposit_token(&account_id, &token_in, amount.into());
                    PromiseOrValue::Value(U128(0))
                },
                TokenReceiverMessage::Execute {call_data, withdraw_address, incognito_address} => {
                    self.call_dapp_2(call_data, withdraw_address, sender_id, incognito_address);
                    PromiseOrValue::Value(U128(0))
                }
            }
        }
    }
}
