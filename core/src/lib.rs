//! Memobi protocol primitives.
//!
//! Consensus-critical functionality will be added here incrementally.
//! The public API is intentionally small while the protocol is experimental.

pub const COIN: u64 = 100_000_000;
pub const MEMO_DECIMALS: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockHeight(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount(pub u64);

impl Amount {
    pub const ZERO: Self = Self(0);
}
