use near_sdk::ext_contract;
use near_sdk::json_types::U128;

#[ext_contract(ext_wnear)]
pub(crate) trait WNearContract {
    fn near_deposit(&mut self);
    fn near_withdraw(&mut self, amount: U128);
}
