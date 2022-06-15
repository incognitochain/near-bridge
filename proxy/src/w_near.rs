use near_sdk::{ext_contract};

#[ext_contract(ext_wnear)]
pub(crate) trait WNearContract {
    fn near_deposit(&mut self);
}

