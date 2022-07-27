/*!
Near - Incognito bridge implementation with JSON serialization.
NOTES:
  - Shield / Unshield features: move tokens forth and back between Near and Incognito
  - Swap beacon
*/

mod token_receiver;
mod errors;
mod utils;
mod w_near;

use std::str;
use std::cmp::Ordering;
use std::convert::{TryInto};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, BorshStorageKey, PanicOnDefault, ext_contract, PromiseResult, AccountId, Promise, PromiseOrValue, serde_json};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::collections::{LookupMap, TreeMap};
use crate::errors::*;
use crate::utils::{
    PROXY_CONTRACT, NEAR_ADDRESS,
    WITHDRAW_INST_LEN, SWAP_COMMITTEE_INST_LEN,
    WITHDRAW_METADATA, SWAP_BEACON_METADATA,
    BURN_METADATA, GAS_FOR_FT_TRANSFER,
    GAS_FOR_EXECUTE, GAS_FOR_WITHDRAW,
    EXECUTE_BURN_PROOF, EXECUTE_BURN_PROOF_METADATA,
    WRAP_NEAR_ACCOUNT, GAS_FOR_WNEAR,
    GAS_FOR_RESOLVE_WNEAR, GAS_FOR_RESOLVE_BRIDGE,
    GAS_FOR_DEPOSIT_AND_EXECUTE
};
use crate::utils::{verify_inst, extract_verifier};
use arrayref::{array_refs, array_ref};
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::json_types::U128;
use crate::w_near::ext_wnear;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct InteractRequest {
    // instruction in bytes
    pub inst: String,
    // beacon height
    pub height: u128,
    // inst paths to build merkle tree
    pub inst_paths: Vec<[u8; 32]>,
    // inst path indicator
    pub inst_path_is_lefts: Vec<bool>,
    // instruction root
    pub inst_root: [u8; 32],
    // blkData
    pub blk_data: [u8; 32],
    // signature index
    pub indexes: Vec<u8>,
    // signatures
    pub signatures: Vec<String>,
    // v value
    pub vs: Vec<u8>
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExecuteRequest {
    pub timestamp: u128,
    pub call_data: String,
    pub incognito_address: String,
    pub token: String,
    pub amount: u128,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct WithdrawRequest {
    pub incognito_address: String,
    pub token: String,
    pub amount: u128,
    pub timestamp: u128,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Transaction,
    BeaconHeight,
    TokenDecimals,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Execute {
    call_data: String
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Vault {
    // mark tx already burn
    pub tx_burn: LookupMap<[u8; 32], bool>,
    // beacon committees
    pub beacons: TreeMap<u128, Vec<String>>,
    // store token decimal
    pub token_decimals: LookupMap<String, u8>,
    // index for new flow
    pub execute_burn_proof_id: u128,
}

// define the methods we'll use on ContractB
#[ext_contract(ext_ft)]
pub trait FtContract {
    fn ft_metadata(&self) -> FungibleTokenMetadata;
    fn ft_balance_of(&mut self, account_id: AccountId) -> U128;
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>, msg: String);
}

#[ext_contract(ext_proxy)]
pub trait ProxyContract {
    fn deposit_near(&mut self, account_id: AccountId, wrap: bool);
    fn call_dapp(&mut self, account_id: AccountId, msg: String) -> (String, U128);
    fn withdraw(
        &mut self,
        token_id: String,
        amount: u128,
        account_id: AccountId,
        incognito_address: String,
        withdraw_address: String,
    ) -> U128;
}

// define methods we'll use as callbacks on ContractA
#[ext_contract(this_contract)]
pub trait Callbacks {
    fn callback_wnear(&mut self, account_id: AccountId, amount: U128);
    fn callback_bridge(&mut self);
    fn callback_deposit(
        &self,
        incognito_address: String,
        token: AccountId,
        amount: u128,
    );
    fn callback_request_withdraw(
        &self,
        incognito_address: String,
        token: AccountId,
    );
    fn callback_execute_burnproof(
        &self,
        incognito_address: String,
        withdraw_addr_str: String,
        source_token: String,
        request_id: String,
    ) -> Promise;
}

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    Deposit { account_id: AccountId },
}

#[near_bindgen]
impl Vault {
    /// Initializes the beacon list
    #[init]
    pub fn new(
        beacons: Vec<String>,
        height: u128,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        assert!(!beacons.len().eq(&0), "Invalid beacon list");
        let mut this = Self {
            tx_burn: LookupMap::new(StorageKey::Transaction), 
            beacons: TreeMap::new(StorageKey::BeaconHeight),
            token_decimals: LookupMap::new(StorageKey::TokenDecimals),
            execute_burn_proof_id: 10,
        };
        // insert beacon height and list in tree
        this.beacons.insert(&height, &beacons);

        this
    }

    /// shield native token
    ///
    /// receive token from users and generate proof
    /// validate proof on Incognito side and mint corresponding token
    #[payable]
    pub fn deposit(
        &mut self,
        incognito_address: String,
    ) {
        let total_native = env::account_balance();
        if total_native.checked_div(1e15 as u128).unwrap_or_default().cmp(&(u64::MAX as u128)) == Ordering::Greater {
            panic!("{}", VALUE_EXCEEDED);
        }

        // extract near amount from deposit transaction
        let amount = env::attached_deposit().checked_div(1e15 as u128).unwrap_or(0);
        env::log_str(format!(
            "{} {} {}",
            incognito_address, NEAR_ADDRESS.to_string(), amount
        ).as_str());
    }

    /// withdraw tokens
    ///
    /// submit burn proof to receive token
    pub fn withdraw(
        &mut self,
        unshield_info: InteractRequest
    ) -> Promise {
        let beacons = self.get_beacons(unshield_info.height);

        // verify instruction
        verify_inst(&unshield_info, beacons);

        // parse instruction
        let inst = hex::decode(unshield_info.inst).unwrap_or_default();
        let inst_ = array_ref![inst, 0, WITHDRAW_INST_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (meta_type, shard_id, token_len, token, receiver_len, receiver_key, _, unshield_amount, tx_id) =
            array_refs![inst_, 1, 1, 1, 64, 1, 64, 24, 8, 32];
        let meta_type = u8::from_be_bytes(*meta_type);
        let shard_id = u8::from_be_bytes(*shard_id);
        let mut unshield_amount = u128::from(u64::from_be_bytes(*unshield_amount));
        let token_len = u8::from_be_bytes(*token_len);
        let receiver_len = u8::from_be_bytes(*receiver_len);
        let token = &token[64 - token_len as usize..];
        let token: String = String::from_utf8(token.to_vec()).unwrap_or_default();
        let receiver_key = &receiver_key[64 - receiver_len as usize..];
        let receiver_key: String = String::from_utf8(receiver_key.to_vec()).unwrap_or_default();

        // validate metatype and key provided
        if (meta_type != WITHDRAW_METADATA) || shard_id != 1 {
            panic!("{}", INVALID_METADATA);
        }

        // check tx burn used
        if self.tx_burn.get(&tx_id).unwrap_or_default() {
            panic!("{}", INVALID_TX_BURN);
        }
        self.tx_burn.insert(&tx_id, &true);

        let account: AccountId = receiver_key.try_into().unwrap();
        if token == NEAR_ADDRESS {
            unshield_amount = unshield_amount.checked_mul(1e15 as u128).unwrap();
            Promise::new(account).transfer(unshield_amount)
        } else {
            let decimals = self.token_decimals.get(&token).unwrap();
            if decimals > 9 {
                unshield_amount = unshield_amount.checked_mul(u128::pow(10, decimals as u32 - 9)).unwrap()
            }
            let token: AccountId = token.try_into().unwrap();
            ext_ft::ext(token)
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .with_attached_deposit(1)
                .ft_transfer(
                    account,
                    U128(unshield_amount),
                    None,
                )
        }
    }

    /// swap beacon committee
    ///
    /// verify old beacon committee's signature and update new beacon committee
    pub fn swap_beacon_committee(
        &mut self,
        swap_info: InteractRequest
    ) -> bool {
        let beacons = self.get_beacons(swap_info.height);

        // verify instruction
        verify_inst(&swap_info, beacons);

        // parse instruction
        let inst = hex::decode(swap_info.inst).unwrap_or_default();
        let inst_ = array_ref![inst, 0, SWAP_COMMITTEE_INST_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (meta_type, shard_id, _, prev_height, _, height, _, num_vals) =
            array_refs![inst_, 1, 1, 16, 16, 16, 16, 16, 16];
        let meta_type = u8::from_be_bytes(*meta_type);
        let shard_id = u8::from_be_bytes(*shard_id);
        let prev_height = u128::from_be_bytes(*prev_height);
        let height = u128::from_be_bytes(*height);
        let num_vals = u128::from_be_bytes(*num_vals);

        let mut beacons: Vec<String> = vec![];
        for i in 0..num_vals {
            let index = i as usize;
            let beacon_key = array_ref![inst, SWAP_COMMITTEE_INST_LEN + index * 32, 32];
            let beacon = hex::encode(beacon_key);
            beacons.push(beacon);
        }

        // validate metatype and key provided
        if meta_type != SWAP_BEACON_METADATA || shard_id != 1 {
            panic!("{}", INVALID_METADATA);
        }

        let my_latest_commitee_height = self.beacons.max().unwrap_or_default();
        assert!(prev_height.eq(&my_latest_commitee_height), "{}", PREV_COMMITTEE_HEIGHT_MISMATCH);
        assert!(height > my_latest_commitee_height, "{}", COMMITTEE_HEIGHT_MISMATCH);

        // swap committee
        self.beacons.insert(&height, &beacons);

        true
    }


    // submit burn proof
    //
    // prepare fund to call contract
    pub fn submit_burn_proof(
        &mut self,
        burn_info: InteractRequest
    ) -> Promise {
        let beacons = self.get_beacons(burn_info.height);

        // verify instruction
        verify_inst(&burn_info, beacons);

        // parse instruction
        let inst = hex::decode(burn_info.inst).unwrap_or_default();
        let inst_ = array_ref![inst, 0, WITHDRAW_INST_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (meta_type, shard_id, token_len, token, receiver_len, receiver_key, _, burn_amount, tx_id) =
            array_refs![inst_, 1, 1, 1, 64, 1, 64, 24, 8, 32];
        let meta_type = u8::from_be_bytes(*meta_type);
        let shard_id = u8::from_be_bytes(*shard_id);
        let burn_amount = u128::from(u64::from_be_bytes(*burn_amount));
        let token_len = u8::from_be_bytes(*token_len);
        let receiver_len = u8::from_be_bytes(*receiver_len);
        let token = &token[64 - token_len as usize..];
        let token: String = String::from_utf8(token.to_vec()).unwrap_or_default();
        let receiver_key = &receiver_key[64 - receiver_len as usize..];
        let receiver_key: String = String::from_utf8(receiver_key.to_vec()).unwrap_or_default();

        // validate metatype and key provided
        if (meta_type != BURN_METADATA) || shard_id != 1 {
            panic!("{}", INVALID_METADATA);
        }

        // check tx burn used
        if self.tx_burn.get(&tx_id).unwrap_or_default() {
            panic!("{}", INVALID_TX_BURN);
        }
        self.tx_burn.insert(&tx_id, &true);
        self.internal_deposit_to_proxy(
            receiver_key,
            token,
            burn_amount
        )
    }

    // call proxy for requesting to execute dapps
    pub fn execute(
        &mut self,
        request: ExecuteRequest,
        signature: String,
        v: u8,
    ) -> Promise {
        let verifier_str = hex::encode(extract_verifier(signature.as_ref(), v, &request));
        self.internal_execute(request, verifier_str)
    }

    pub fn request_withdraw(
        &mut self,
        request: WithdrawRequest,
        signature: String,
        v: u8,
    ) -> Promise {
        let verifier_str = hex::encode(extract_verifier(signature.as_ref(), v, &request));
        self.internal_request_withdraw(request, verifier_str, "".to_string())
    }

    #[private]
    fn internal_execute(
        &mut self,
        request: ExecuteRequest,
        verifier_str: String,
    ) -> Promise {
        let verifier_id: AccountId = verifier_str.try_into().unwrap();
        let proxy_id: AccountId = PROXY_CONTRACT.to_string().try_into().unwrap();

        ext_proxy::ext(proxy_id)
            .with_static_gas(GAS_FOR_EXECUTE)
            .call_dapp(
                verifier_id,
                request.call_data,
            )
    }

    #[private]
    fn internal_request_withdraw(
        &mut self,
        request: WithdrawRequest,
        verifier_str: String,
        withdraw_str: String,
    ) -> Promise {
        let verifier_id: AccountId = verifier_str.try_into().unwrap();

        let proxy_id: AccountId = PROXY_CONTRACT.to_string().try_into().unwrap();
        let _token_id: AccountId = request.token.clone().try_into().unwrap();

        ext_proxy::ext(proxy_id)
            .with_static_gas(GAS_FOR_WITHDRAW)
            .withdraw(
                request.token.clone(),
                request.amount,
                verifier_id,
                request.incognito_address,
                withdraw_str,
            )
    }

    #[private]
    fn internal_deposit_to_proxy(
        &mut self,
        receiver_key: String,
        token: String,
        amount: u128
    ) -> Promise {
        let account: AccountId = receiver_key.clone().try_into().unwrap();
        let proxy: AccountId = PROXY_CONTRACT.to_string().try_into().unwrap();
        if token == NEAR_ADDRESS {
            ext_proxy::ext(proxy)
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .with_attached_deposit(amount)
                .deposit_near(
                    account,
                    true,
                ).into()
        } else {
            let token: AccountId = token.try_into().unwrap();
            let msg = serde_json::to_string(
                &TokenReceiverMessage::Deposit {
                    account_id: account
                }
            ).unwrap_or_default();
            ext_ft::ext(token)
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .with_attached_deposit(1)
                .ft_transfer_call(
                    proxy,
                    U128(amount),
                    None,
                    msg,
                ).into()
        }
    }

    /// execute request from beacon
    pub fn execute_with_burn_proof(
        &mut self,
        burn_info: InteractRequest
    ) -> Promise {
        let beacons = self.get_beacons(burn_info.height);

        // verify instruction
        verify_inst(&burn_info, beacons);

        // parse instruction
        let inst = hex::decode(burn_info.inst).unwrap_or_default();
        assert!(inst.len() > EXECUTE_BURN_PROOF, "Invalid beacon instruction");
        let inst_ = array_ref![inst, 0, EXECUTE_BURN_PROOF];
        // extract data from instruction
        // removed external call address
        // layout: meta(1), shard(1), network(1), extToken(32), amount(32), txID(32), withdrawAddr(32), redepositAddr(101), extCalldata(*)
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            meta_type, shard_id, _,
            token_len, token,
            _, amount, tx_id,
            withdraw_addr_len, withdraw_addr,
            redeposit_addr
        ) = array_refs![inst_, 1, 1, 1, 1, 64, 24, 8, 32, 1, 64, 101];
        let meta_type = u8::from_be_bytes(*meta_type);
        let shard_id = u8::from_be_bytes(*shard_id);

        let token_len = u8::from_be_bytes(*token_len);
        let token = &token[64 - token_len as usize..];
        let token: String = String::from_utf8(token.to_vec()).unwrap();

        let mut amount = u128::from(u64::from_be_bytes(*amount));
        if token == NEAR_ADDRESS {
            amount = amount.checked_mul(1e15 as u128).unwrap();
        } else {
            let decimals = self.token_decimals.get(&token).unwrap();
            if decimals > 9 {
                amount = amount.checked_mul(u128::pow(10, decimals as u32 - 9)).unwrap()
            }
        }

        let withdraw_addr_len = u8::from_be_bytes(*withdraw_addr_len);
        let withdraw_addr = &withdraw_addr[64 - withdraw_addr_len as usize..];
        let mut withdraw_addr: String = String::from_utf8(withdraw_addr.to_vec()).unwrap_or_default();

        let redeposit_addr: String = match str::from_utf8(redeposit_addr) {
            Ok(v) => v.to_string(),
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        let call_data = &inst.as_slice()[EXECUTE_BURN_PROOF as usize..];
        // verify
        // validate metatype and key provided
        if (meta_type != EXECUTE_BURN_PROOF_METADATA) || shard_id != 1 {
            panic!("{}", INVALID_METADATA);
        }

        // check tx burn used
        if self.tx_burn.get(&tx_id).unwrap_or_default() {
            panic!("{}", INVALID_TX_BURN);
        }
        self.tx_burn.insert(&tx_id, &true);
        if withdraw_addr_len == 0 {
            withdraw_addr = "".to_string();
        }

        let execute_data = ExecuteRequest {
            timestamp: 0,
            call_data: match str::from_utf8(call_data) {
                Ok(v) => v.to_string(),
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            },
            incognito_address: redeposit_addr.clone(),
            token,
            amount
        };

        // convert to w-near if move native near
        if token == NEAR_ADDRESS {
            let wnear_id: AccountId = WRAP_NEAR_ACCOUNT.to_string().try_into().unwrap();
            let near_deposit_ps = ext_wnear::ext(wnear_id)
                .with_static_gas(GAS_FOR_WNEAR)
                .with_attached_deposit(amount)
                .near_deposit();

            near_deposit_ps.then(
                Self::ext(env::current_account_id().clone())
                    .with_static_gas(GAS_FOR_RESOLVE_WNEAR)
                    .callback_wnear(
                        execute_data,
                        U128(amount),
                    )
            ).into()
        } else {
            self.internal_transfer_execute(execute_data)
        }
    }

    /// transfer and execute
    #[private]
    fn internal_transfer_execute(
        &mut self,
        execute_data: ExecuteRequest
    ) -> Promise {
        let obj = Execute {
            call_data: execute_data.call_data,
        };
        let mut token_id: AccountId;
        if execute_data.token == "" {
            token_id = WRAP_NEAR_ACCOUNT.to_string().try_into().unwrap();
        } else {
            token_id = execute_data.token.try_into().unwrap();
        }

        let mut msg = serde_json::to_string(&obj).unwrap();
        let receiver:AccountId = PROXY_CONTRACT.to_string().try_into().unwrap();

        ext_ft::ext(token_id.clone())
            .with_static_gas(GAS_FOR_DEPOSIT_AND_EXECUTE)
            .with_attached_deposit(1)
            .ft_transfer_call(
                receiver,
                U128(execute_data.amount),
                None,
                msg,
            ).then(
            Self::ext(env::current_account_id().clone())
                .with_static_gas(GAS_FOR_RESOLVE_BRIDGE)
                .callback_bridge()
        ).into()
    }

    /// getters

    /// get beacon list by height
    pub fn get_beacons(&self, height: u128) -> Vec<String> {
        let get_height_key = self.beacons.lower(&(height + 1)).unwrap();
        self.beacons.get(&get_height_key).unwrap()
    }

    /// check tx burn used
    pub fn get_tx_burn_used(self, tx_id: &[u8; 32]) -> bool {
        self.tx_burn.get(tx_id).unwrap_or_default()
    }

    /// callbacks
    #[private]
    pub fn callback_wnear(&mut self, execute_data: ExecuteRequest, amount: U128) -> PromiseOrValue<U128> {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");
        // handle the result from the first cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                // extract near amount from deposit transaction
                let amount = amount.0.checked_div(1e15 as u128).unwrap_or(0);
                env::log_str(format!(
                    "{} {} {}",
                    execute_data.incognito_address, NEAR_ADDRESS.to_string(), amount
                ).as_str());
                PromiseOrValue::Value(U128(0))
            },
            PromiseResult::Successful(_result) => {
                let transfer_execute_ps: Promise = self.internal_transfer_execute(execute_data);
                PromiseOrValue::Promise(transfer_execute_ps)
            }
        }
    }

    #[private]
    pub fn callback_bridge(&mut self) -> PromiseOrValue<U128> {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");
        // handle the result from the first cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                // extract near amount from deposit transaction
                let amount = amount.checked_div(1e15 as u128).unwrap_or(0);
                env::log_str(format!(
                    "{} {} {}",
                    execute_data.incognito_address, NEAR_ADDRESS.to_string(), amount
                ).as_str());
                PromiseOrValue::Value(U128(0))
            },
            PromiseResult::Successful(_result) => {
                PromiseOrValue::Value(U128(0))
            }
        }
    }

    #[private]
    pub fn callback_deposit(&mut self, incognito_address: String, token: AccountId, amount: u128) -> PromiseOrValue<U128> {
        assert_eq!(env::promise_results_count(), 2, "This is a callback method");

        // handle the result from the second cross contract call this method is a callback for
        let token_meta_data: FungibleTokenMetadata = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("{:?}", b"Unable to make comparison"),
            PromiseResult::Successful(result) => near_sdk::serde_json::from_slice::<FungibleTokenMetadata>(&result)
                .unwrap()
                .into(),
        };

        // handle the result from the first cross contract call this method is a callback for
        let mut vault_acc_balance: u128 = match env::promise_result(1) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("{:?}", b"Unable to make comparison"),
            PromiseResult::Successful(result) => near_sdk::serde_json::from_slice::<U128>(&result)
                .unwrap()
                .into(),
        };

        let mut emit_amount = amount;
        if token_meta_data.decimals > 9 {
            emit_amount = amount.checked_div(u128::pow(10, (token_meta_data.decimals - 9) as u32)).unwrap_or_default();
            vault_acc_balance = vault_acc_balance.checked_div(u128::pow(10, (token_meta_data.decimals - 9) as u32)).unwrap_or_default();
        }

        if vault_acc_balance.cmp(&(u64::MAX as u128)) == Ordering::Greater {
            panic!("{}", VALUE_EXCEEDED)
        }

        let decimals_stored = self.token_decimals.get(&token.to_string()).unwrap_or_default();
        if decimals_stored == 0 {
            self.token_decimals.insert(&token.to_string(), &token_meta_data.decimals);
        }

        env::log_str(
            format!(
                "{} {} {}",
                incognito_address, token, emit_amount
            ).as_str());

        PromiseOrValue::Value(U128(0))
    }

    #[private]
    pub fn callback_execute_burnproof(
        &mut self,
        incognito_address: String,
        withdraw_addr_str: String,
        source_token: String,
        request_id: String,
    ) -> Promise {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        // handle the result from the second cross contract call this method is a callback for
        let execute_result: (String, U128) = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => panic!("{:?}", b"Unable to make comparison"),
            PromiseResult::Successful(result) => serde_json::from_slice::<(String, U128)>(&result)
                .unwrap()
                .into(),
        };

        // withdraw request
        let withdraw_data = WithdrawRequest {
            incognito_address,
            amount: execute_result.1.0,
            token: execute_result.0.clone(),
            timestamp: 0,
        };

        let mut temp = withdraw_addr_str;
        if source_token == execute_result.0 {
            temp = "".to_string();
        }

        self.internal_request_withdraw(
            withdraw_data,
            request_id,
            temp,
        )
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::{serde_json};
    use ethsign::{Protected, SecretKey};
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance, MockedBlockchain};

    /// Creates contract and a pool with tokens with 0.3% of total fee.
    fn setup_contract() -> (VMContextBuilder, Vault) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Vault::new(
            vec!["45e6d8d759bc5993097236e5f2d17053969f0b769bb1d0f8e222b6c40a0f6af345e6d8d759bc5993097236e5f2d17053969f0b769bb1d0f8e222b6c40a0f6af3".to_string()],
            0
        );
        (context, contract)
    }

    fn to_32_bytes(hex_str: &str) -> [u8; 32] {
        let bytes = hex::decode(hex_str).unwrap();
        let mut bytes_ = [0u8; 32];
        bytes_.copy_from_slice(&bytes);
        bytes_
    }

    #[test]
    fn test_serialize() {
        let msg_obj = InteractRequest {
            inst: "cuongcute".to_string(),
            height: 16,
            inst_paths: vec![to_32_bytes("23abf9d3acf3fde6246cce9e392c2154ab8423d8d2e01053e74db7f6d17aea4f")],
            inst_path_is_lefts: vec![false],
            inst_root: to_32_bytes("45e6d8d759bc5993097236e5f2d17053969f0b769bb1d0f8e222b6c40a0f6af3"),
            blk_data: to_32_bytes("eff9f595401e37992a3a1fb0c1908e0d4bb2105eae42c0ef6499483b991f2c91"),
            indexes: vec![0, 1, 2, 3],
            signatures: vec![
                "3ba689cfbcbfe81d10f47c0becd911ece7fd1c99ce3bf84c61cf20f3bfc2979438251b39a913e934bd6b61def19fac8da98808cce9b8f428809885364a49d81c".to_string(),
                "fbb1705370519af0e89fa86ced533123a8a33db842a3d90a7c8c69ee82ce20c44ecf63c0f1646d7f2d173b7d4dae99c16e29af1bedcc5ee1a88e15c132f27136".to_string(),
                "0cb23956deaaf8070c9dbc36e2035d1b641112d8b75187c7ee834f1dd00adf165c2a88fc3a356c795f6e4df4cf52c81f091d7a4fde215dba1eec47768da7b7ae".to_string(),
                "6801dc29a7d1784f57c511369f84d68f04630bc7afcaa2b92c03272af26430fb7b93aaae22ce4f44818acb3345db276252ef71c7442cf1fe94d1d230191208cb".to_string(),
            ],
            vs: vec![0, 0, 1, 1],
        };
        let msg_str = serde_json::to_string(&msg_obj).unwrap();
        println!("{}", msg_str);
    }

    #[test]
    fn test_serialize_withdraw_request() {
        let request = WithdrawRequest {
            incognito_address: "12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2YFpctHNaDdkSiQshsjw2UFUuwdEvcidgaKmF3VJpY5f8RdN".to_string(),
            token: "cuongcute.testnet".to_string(),
            timestamp: 123,
            amount: 1000_000_000,
        };

        let private_key = hex::decode("2a3526dd05ad2ebba87673f711ef8c336115254ef8fcd38c4d8166db9a8120e4").unwrap();
        let secret = SecretKey::from_raw(private_key.as_slice()).unwrap();
        let message = serde_json::to_string(&request).unwrap();
        let data = env::keccak256(message.as_bytes());

        // Sign the message
        let signature = secret.sign(data.as_slice()).unwrap();
        println!("{:?}", signature);

        // Recover the signer
        let public = signature.recover(data.as_slice()).unwrap();
        println!("{:?}", public);

        // Verify the signature
        let res = public.verify(&signature, data.as_slice()).unwrap();
        assert!(res);

        let signature_str = [signature.r, signature.s].concat();
        let result = extract_verifier(hex::encode(signature_str.as_slice()).as_str()
                                      , signature.v, &request);

        print!("Actual {:?} \n", hex::encode(result));
        print!("Expect {:?}", hex::encode(secret.public().address()));
        assert_eq!(hex::encode(result), hex::encode(secret.public().address()));
    }

    #[test]
    fn test_execute_burn_proof() {
        let executeRequest = InteractRequest {
            inst: "a001010c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000777261702e746573746e6574000000000000000000000000000000000000000000000000851812a357a1339d0baceab06e95c52314f6792b2f5e6fd4ce5b583aeb63572f6a75bc56d820de66000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000031327376666b5036773555444a445343777148393738507671697142784b6d556e4139656d3979415957594a565276377775585931716868597050416d3442447a326d4c624672526d644b337952686e54714a435a584b48556d6f69374e563833484348327b22616374696f6e223a7b22706f6f6c5f6964223a35342c22746f6b656e5f696e223a22777261702e746573746e6574222c22616d6f756e745f696e223a2239353930343335383939323434363232373439222c22746f6b656e5f6f7574223a22757364632e66616b65732e746573746e6574222c226d696e5f616d6f756e745f6f7574223a2231227d2c226163636f756e745f6964223a2234393661646432633234653137373131643935313231373239303162353530326466333765313034393364323437633337316562386463336534623137336663227d".to_string(),
            height: 304,
            inst_paths: vec![
                to_32_bytes("82a8c4d7dcdcf1e28ec58e7218155c8f2e75cdc2aded968d63da53efb8848abb")
            ],
            inst_path_is_lefts: vec![
                false
            ],
            inst_root: to_32_bytes("fd64d3bd7f578bbb58ee9088949d96f4186b04b3d4b5751ce0104399d7ba4b7c"),
            blk_data: to_32_bytes("b2f85d2ee41b2fc42a7e06dc90ca5ddec6d3b08e84a97519c9f9709315155681"),
            indexes: vec![1, 2, 3],
            signatures: vec![
                "af503e8cc61c73d6ae5728d5b37d04ec7fa7aff190040cbcffc213c7e2046e721a8d3fe9c12e62e9802f1bedaf1d38b58ea11ed2e38ec4301dd798c5be4ae469".to_string(),
                "869f5225d790484190ec4bf5113dadc568e8232c90bb711909f3154ff54b477c3b96040e5d041e76dccad008c6453fdfbee8295119bf641f7b9bee8e0d79aa6d".to_string(),
                "c3622371e7355ab5e4f48097e0e39b435b444f0557b6269c55cd427aac932fb327f5592748f839bbb98666613ebcb1e7100ea02f26a2becb960ba1ef8d909460".to_string(),
            ],
            vs: vec![1, 0, 0],
        };

        let (_, mut contract) = setup_contract();
        contract.execute_with_burn_proof(executeRequest);
    }
}
