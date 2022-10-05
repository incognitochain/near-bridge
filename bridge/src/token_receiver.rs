use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Serialize};
use near_sdk::{serde_json, env, PromiseOrValue};
use near_sdk::AccountId;
use near_sdk::json_types::U128;

use crate::errors::*;
use crate::*;

use crate::utils::{GAS_FOR_RESOLVE_DEPOSIT, GAS_FOR_RETRIEVE_INFO};


/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum TokenReceiverMessage {
    Deposit {
        incognito_address: String,
        tx: String,
        signature: String,
    },
}

#[near_bindgen]
impl FungibleTokenReceiver for Vault {
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
        if msg.is_empty() {
            panic!("{}", INVALID_MESSAGE)
        }
        // shield request
        let message =
            serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR28_WRONG_MSG_FORMAT);
        match message {
            TokenReceiverMessage::Deposit {
                incognito_address,
                tx,
                signature,
            } => {
                // verify regulator signature
                assert!(verify_regulator(
                    ShieldInfo {
                        sender: sender_id.to_string(),
                        tx
                    },
                    signature
                ), "{}", INVALID_REGULATOR_SIG);

                let amount = amount.0;
                let ft_metadata_ps = ext_ft::ext(token_in.clone())
                    .with_static_gas(GAS_FOR_RETRIEVE_INFO)
                    .ft_metadata();

                let ft_balance_of_ps = ext_ft::ext(token_in.clone())
                    .with_static_gas(GAS_FOR_RETRIEVE_INFO)
                    .ft_balance_of(env::current_account_id().clone());
                ft_metadata_ps.and(
                    ft_balance_of_ps
                ).then(
                    Self::ext(env::current_account_id().clone())
                        .with_static_gas(GAS_FOR_RESOLVE_DEPOSIT)
                        .callback_deposit(
                            incognito_address,
                            token_in,
                            amount,
                        )
                ).into()
            }
        }
    }
}



#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let msg_obj: TokenReceiverMessage = TokenReceiverMessage::Deposit {
            incognito_address: "my_address".to_string(),
            tx: "65bQNcfAKdfLzZZFsW9KECnQ8JFADQFocMEtTapkEpbp".to_string(),
            signature: "6798a44fe4a7f7d496d38964ffd0235a368937330418bd96f0ba65141dc88c92361d387cddddf9f4f5e0d7ce08fedd75947a69db6a3b0f5e4e6d596a33bd96e801".to_string(),
        };
        let msg_str = serde_json::to_string(&msg_obj).unwrap();
        println!("{}", msg_str);
    }

    #[test]
    fn test_deserialize() {
        let msg_str = r#"{"incognito_address":"my_address"}"#;
        let msg_obj: TokenReceiverMessage = serde_json::from_str(&msg_str).unwrap();
        println!("{:?}", msg_obj);
    }
}