use near_sdk::Gas;

pub const BRIDGE_CONTRACT: &str = "0baceab06e95c52314f6792b2f5e6fd4ce5b583aeb63572f6a75bc56d820de66";
pub const WRAP_NEAR_ACCOUNT: &str = "wrap.testnet";
pub const REF_FINANCE_ACCOUNT: &str = "ref-finance-101.testnet";

pub const GAS_FOR_WNEAR: Gas = Gas(10_000_000_000_000);
pub const GAS_FOR_RESOLVE_WNEAR: Gas = Gas(10_000_000_000_000);

pub const GAS_FOR_RESOLVE_DEPOSIT_REF_FINANCE: Gas = Gas(139_000_000_000_000);
pub const GAS_FOR_SWAP_REF_FINANCE: Gas = Gas(40_000_000_000_000);
pub const GAS_FOR_WITHDRAW_REF_FINANCE: Gas = Gas(57_000_000_000_000);
// deposit - swap - withdraw gas
pub const GAS_FOR_RESOLVE_DEPOSIT_SWAP_WITHDRAW: Gas = Gas(170_000_000_000_000);

pub const GAS_FOR_DEPOSIT_BRIDGE: Gas = Gas(52_000_000_000_000);
pub const GAS_FOR_DEPOSIT_REF: Gas = Gas(31_000_000_000_000);