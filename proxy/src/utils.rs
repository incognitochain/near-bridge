use near_sdk::Gas;

pub const BRIDGE_CONTRACT: &str = "bridge.incognito.testnet";
pub const WRAP_NEAR_ACCOUNT: &str = "wrap.testnet";
pub const REF_FINANCE_ACCOUNT: &str = "ref-finance-101.testnet";

pub const GAS_FOR_WNEAR: Gas = Gas(10_000_000_000_000);
pub const GAS_FOR_RESOLVE_WNEAR: Gas = Gas(10_000_000_000_000);

pub const GAS_FOR_SWAP_REF_FINANCE: Gas = Gas(40_000_000_000_000);
pub const GAS_FOR_RESOLVE_SWAP_REF_FINACE: Gas = Gas(30_000_000_000_000);
pub const GAS_FOR_WITHDRAW_REF_FINANCE: Gas = Gas(10_000_000_000_000);
pub const GAS_FOR_RESOLVE_WITHDRAW_REF_FINACE: Gas = Gas(10_000_000_000_000);

pub const GAS_FOR_DEPOSIT: Gas = Gas(20_000_000_000_000);
pub const GAS_FOR_RESOLVE_DEPOSIT: Gas = Gas(20_000_000_000_000);