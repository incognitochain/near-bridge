use near_sdk::Gas;

pub const BRIDGE_CONTRACT: &str = "bridge.incognito.testnet";
pub const WRAP_NEAR_ACCOUNT: &str = "wrap.testnet";
pub const REF_FINANCE_ACCOUNT: &str = "ref-finance-101.testnet";

pub const GAS_FOR_WNEAR: Gas = Gas(10_000_000_000_000);
pub const GAS_FOR_RESOLVE_WNEAR: Gas = Gas(10_000_000_000_000);