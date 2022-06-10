mod errors;
mod token_receiver;
mod utils;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, serde_json, PanicOnDefault};
use utils::WRAP_NEAR_ACCOUNT;

use crate::errors::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub enum RunningState {
    Running,
    Paused,
}

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    DappRequest {
        dapp_account: String,
        payload: String,
    },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Proxy {
    pub state: RunningState,
}

#[near_bindgen]
impl Proxy {
    #[init]
    pub fn new() -> Self {
        Proxy {
            state: RunningState::Running,
        }
    }

    #[payable]
    pub fn call_dapp(&mut self, msg: String) {
        let amount = env::attached_deposit();
        let message =
            serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR28_WRONG_MSG_FORMAT);
        match message {
            TokenReceiverMessage::DappRequest {
                dapp_account,
                payload,
            } => {
                if dapp_account == WRAP_NEAR_ACCOUNT {
                    
                } else {
                    panic!("{}", INVALID_MESSAGE)
                }
            }
        }
    }
}

/// Internal methods implementation.
impl Proxy {
    fn assert_contract_running(&self) {
        match self.state {
            RunningState::Running => (),
            _ => panic!("Contract is paused"),
        };
    }
}
