//! Memobi protocol primitives.
//!
//! Consensus-critical functionality is being added incrementally. Public APIs
//! stay small while the protocol remains experimental.

pub mod block;
pub mod chain;
pub mod codec;
pub mod difficulty;
pub mod hash;
pub mod p2p;
pub mod reward;
pub mod pow;
pub mod transaction;
pub mod utxo;

pub use hash::Hash32;
pub use transaction::{OutPoint, Transaction, TxInput, TxOutput};

pub const COIN: u64 = 100_000_000;
pub const MEMO_DECIMALS: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockHeight(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount(pub u64);

impl Amount {
    pub const ZERO: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    UnexpectedEof,
    TrailingBytes,
    LengthOverflow,
    InvalidMessageVersion,
    InvalidMessageType,
}
