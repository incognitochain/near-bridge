mod account;
mod errors;
mod token_receiver;
mod utils;

use account::Account;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, serde_json, AccountId, BorshStorageKey, PanicOnDefault};
use utils::WRAP_NEAR_ACCOUNT;

use crate::errors::*;
use crate::utils::BRIDGE_CONTRACT;

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

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Account,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Proxy {
    accounts: LookupMap<AccountId, Account>,
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
    pub fn call_dapp(&mut self, msg: String) {
        let sender_id = env::predecessor_account_id();
        assert_eq!(sender_id.to_string(), BRIDGE_CONTRACT);
        
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
impl Proxy {}
