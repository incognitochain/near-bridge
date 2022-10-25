/*!
Near - Incognito bridge implementation with JSON serialization.
NOTES:
  - Shield / Unshield features: move tokens forth and back between Near and Incognito
  - Swap beacon
*/

extern crate core;

mod token_receiver;
mod errors;
mod utils;

use std::str;
use std::cmp::Ordering;
use std::convert::{TryInto};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, BorshStorageKey, PanicOnDefault, ext_contract, PromiseResult, AccountId, Promise, PromiseOrValue, StorageUsage, Balance};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::collections::{LookupMap, TreeMap};
use crate::errors::*;
use crate::utils::*;
use arrayref::{array_refs, array_ref};
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::json_types::U128;


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
pub struct ShieldInfo {
    pub sender: String,
    pub tx: String,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Transaction,
    BeaconHeight,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Vault {
    // mark tx already burn
    pub tx_burn: LookupMap<[u8; 32], bool>,
    // beacon committees
    pub beacons: TreeMap<u128, Vec<String>>,
    // unshield storage
    pub unshield_storage_usage:  StorageUsage,
    // storage usage per beacon
    pub beacon_storage_usage:  StorageUsage,
    // contract pause status
    pub is_paused: bool,
    // contract admin
    pub admin: AccountId
}

// define the methods we'll use on ContractB
#[ext_contract(ext_ft)]
pub trait FtContract {
    fn ft_metadata(&self) -> FungibleTokenMetadata;
    fn ft_balance_of(&mut self, account_id: AccountId) -> U128;
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

// define methods we'll use as callbacks on ContractA
#[ext_contract(this_contract)]
pub trait Callbacks {
    fn callback_deposit(
        &self,
        incognito_address: String,
        token: AccountId,
        amount: u128,
    );
    fn callback_withdraw(
        &self,
        tx_id: &[u8; 32],
        token: AccountId,
        account: AccountId,
        unshield_amount: u128,
    ) -> PromiseOrValue<U128>;
    fn callback_resolve_withdraw(
        &self,
        tx_id: &[u8; 32],
    );
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
            unshield_storage_usage: env::storage_usage(),
            beacon_storage_usage: env::storage_usage(),
            is_paused: false,
            admin: env::predecessor_account_id(),
        };
        let initial_storage_usage = env::storage_usage();
        // calculate near needed on each action
        // unshield
        let tx_id : &[u8; 32] = &[0xff; 32];
        this.tx_burn.insert(&tx_id, &true);
        this.unshield_storage_usage = env::storage_usage() - initial_storage_usage;
        this.tx_burn.remove(&tx_id);
        // beacon
        let beacon = vec!["a".repeat(64)];
        this.beacons.insert(&1, &beacon);
        this.beacon_storage_usage = env::storage_usage() - initial_storage_usage;
        this.beacons.remove(&1);
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
        tx: String,
        signature: String,
    ) {
        assert!(verify_regulator(
            ShieldInfo {
                    sender: env::predecessor_account_id().to_string(),
                    tx
                },
                signature
        ), "{}", INVALID_REGULATOR_SIG);

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
    #[payable]
    pub fn withdraw(
        &mut self,
        unshield_info: InteractRequest
    ) -> Promise {
        // check storage staking first
        let minimum_amount = Balance::from(self.unshield_storage_usage) * env::storage_byte_cost();
        if env::attached_deposit() < minimum_amount {
            env::panic_str(format!("The attached deposit is less than the minimum storage balance, need at least {}", minimum_amount).as_str());
        }

        let beacons = self.get_beacons(unshield_info.height);

        // verify instruction
        verify_inst(&unshield_info, beacons);

        // parse instruction
        let inst = hex::decode(unshield_info.inst).unwrap_or_default();
        let inst_ = array_ref![inst, 0, WITHDRAW_INST_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (meta_type, shard_id, token_len, token, receiver_len, receiver_key, _, unshield_amount, tx_id) =
            array_refs![inst_, 1, 1, 1, 67, 1, 64, 24, 8, 32];
        let meta_type = u8::from_be_bytes(*meta_type);
        let shard_id = u8::from_be_bytes(*shard_id);
        let mut unshield_amount = u128::from(u64::from_be_bytes(*unshield_amount));
        // remove 3 byte prefix NER
        let token_len = u8::from_be_bytes(*token_len).checked_sub(3).unwrap_or_else(|| env::panic_str("invalid token length"));
        let token = &token[67 - token_len as usize..];
        let token: String = String::from_utf8(token.to_vec()).unwrap_or_default();
        let receiver_len = u8::from_be_bytes(*receiver_len);
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
            let token: AccountId = token.try_into().unwrap();
            ext_ft::ext(token.clone())
                .with_static_gas(GAS_FOR_RETRIEVE_INFO)
                .ft_metadata()
                .then(
                    Self::ext(env::current_account_id().clone())
                    .with_static_gas(GAS_FOR_WITHDRAW)
                    .callback_withdraw(
                        tx_id,
                        token,
                        account,
                        unshield_amount,
                    )
                ).into()
        }
    }

    /// swap beacon committee
    ///
    /// verify old beacon committee's signature and update new beacon committee
    #[payable]
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

        // check storage staking
        let minimum_amount = Balance::from(self.beacon_storage_usage) * env::storage_byte_cost() * num_vals;
        if env::attached_deposit() < minimum_amount {
            env::panic_str(format!("The attached deposit is less than the minimum storage balance, need at least {}", minimum_amount).as_str());
        }

        let mut beacons: Vec<String> = vec![];
        for i in 0..num_vals {
            let index = i as usize;
            let beacon_key = array_ref![inst, SWAP_COMMITTEE_INST_LEN + index * 32, 32];
            let beacon = hex::encode(&beacon_key[12..]);
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

    /// get storage cost unit on swap and unshield actions
    pub fn get_storage(self) -> (StorageUsage, StorageUsage) {
        return (self.beacon_storage_usage, self.unshield_storage_usage)
    }

    /// callbacks

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

        env::log_str(
            format!(
                "{} {} {}",
                incognito_address, token, emit_amount
            ).as_str());

        PromiseOrValue::Value(U128(0))
    }

    #[private]
    pub fn callback_withdraw(
        &mut self,
        tx_id: &[u8; 32],
        token: AccountId,
        account: AccountId,
        unshield_amount: u128,
    ) -> PromiseOrValue<U128> {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        // handle the result from the second cross contract call this method is a callback for
        let (token_meta_data, success) = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => (
                FungibleTokenMetadata{
                    spec: "".to_string(),
                    name: "".to_string(),
                    symbol: "".to_string(),
                    icon: None,
                    reference: None,
                    reference_hash: None,
                    decimals: 0,
                }, 
                false
            ),
            PromiseResult::Successful(result) => (
                near_sdk::serde_json::from_slice::<FungibleTokenMetadata>(&result).unwrap(),
                true
            ),
        };

        if !success {
            // unmark withdraw
            self.tx_burn.remove(&tx_id);

            return PromiseOrValue::Value(U128(0));
        }

        let mut amount = unshield_amount;
        if token_meta_data.decimals > 9 {
            amount = amount.checked_mul(u128::pow(10, token_meta_data.decimals as u32 - 9)).unwrap()
        }

        ext_ft::ext(token)
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .with_attached_deposit(1)
            .ft_transfer(
                account,
                U128(amount),
                None,
            ).then(
            Self::ext(env::current_account_id().clone())
                .with_static_gas(GAS_FOR_RESOLVE_WITHDRAW)
                .callback_resolve_withdraw(
                    tx_id
                )
            ).into()
    }

    #[private]
    pub fn callback_resolve_withdraw(
        &mut self,
        tx_id: &[u8; 32],
    ) {
        assert_eq!(env::promise_results_count(), 1, "This is a callback method");

        let withdraw_success: bool = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => false,
            PromiseResult::Successful(_result) => true,
        };

        if !withdraw_success {
            // unmark withdraw and check withdraw gas provided
            self.tx_burn.remove(&tx_id);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use ethsign::{SecretKey};
    use near_contract_standards::fungible_token::FungibleToken;
    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
    use near_contract_standards::non_fungible_token::Token;
    use near_sdk::{serde_json};
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env};
    use crate::token_receiver::TokenReceiverMessage;

    fn to_32_bytes(hex_str: &str) -> [u8; 32] {
        let bytes = hex::decode(hex_str).unwrap();
        let mut bytes_ = [0u8; 32];
        bytes_.copy_from_slice(&bytes);
        bytes_
    }

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
    fn test_verify_regulator() {
        let request = ShieldInfo {
            sender: "incognito.deployer.testnet".to_string(),
            tx: "65bQNcfAKdfLzZZFsW9KECnQ8JFADQFocMEtTapkEpbp".to_string(),
        };
        let private_key = hex::decode("98452cb9c013387c2f5806417fe198a0de014594678e2f9d3223d7e7e921b04d").unwrap();
        let secret = SecretKey::from_raw(private_key.as_slice()).unwrap();
        let message = serde_json::to_string(&request).unwrap();
        let data = env::keccak256(message.as_bytes());
        // Sign the message
        let signature = secret.sign(data.as_slice()).unwrap();
        let regulator_sig = format!("{}{}{}{}", hex::encode(signature.r), hex::encode(signature.s), 0, signature.v);
        println!("Regulator signature {:?}", regulator_sig);

        assert_eq!(verify_regulator(request, regulator_sig), true)
    }

    #[test]
    fn test_compliance_shield() {
        let request = ShieldInfo {
            sender: accounts(0).to_string(),
            tx: "65bQNcfAKdfLzZZFsW9KECnQ8JFADQFocMEtTapkEpbp".to_string(),
        };
        let private_key = hex::decode("98452cb9c013387c2f5806417fe198a0de014594678e2f9d3223d7e7e921b04d").unwrap();
        let secret = SecretKey::from_raw(private_key.as_slice()).unwrap();
        let message = serde_json::to_string(&request.clone()).unwrap();
        let data = env::keccak256(message.as_bytes());
        let signature = secret.sign(data.as_slice()).unwrap();
        let regulator_sig = format!("{}{}{}{}", hex::encode(signature.r), hex::encode(signature.s), 0, signature.v);
        let (mut context, mut contract) = setup_contract();
        let incognito_addr = "12svfkP6w5UDJDSCwqH978PvqiqBxKmUnA9em9yAYWYJVRv7wuXY1qhhYpPAm4BDz2mLbFrRmdK3yRhnTqJCZXKHUmoi7NV83HCH2YFpctHNaDdkSiQshsjw2UFUuwdEvcidgaKmF3VJpY5f8RdN";
        contract.deposit(
            incognito_addr.to_string(),
            request.clone().tx,
            regulator_sig.clone(),
        );

        // test shield FT
        let msg_obj: TokenReceiverMessage = TokenReceiverMessage::Deposit {
            incognito_address: incognito_addr.to_string(),
            tx: request.clone().tx,
            signature: regulator_sig,
        };

        let msg_str = serde_json::to_string(&msg_obj).unwrap();
        contract.ft_on_transfer(
            AccountId::new_unchecked(request.sender),
            U128(100),
            msg_str
        );
    }
}
