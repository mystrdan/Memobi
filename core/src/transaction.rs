//! Native UTXO transaction primitives.
//!
//! Spending-condition and signature formats remain intentionally opaque until
//! the cryptographic/addressing design is finalized.

use crate::{
    ProtocolError,
    codec::{Encode, Reader, put_bytes, put_u32_le, put_u64_le, read_bytes_u32},
    hash::{Hash32, sha256},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutPoint {
    pub txid: Hash32,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxInput {
    pub previous_output: OutPoint,
    pub unlocking_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxOutput {
    pub value: u64,
    pub spending_condition: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub version: u32,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub fee: u64,
}

impl Encode for OutPoint {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtocolError> {
        out.extend_from_slice(self.txid.as_bytes());
        put_u32_le(out, self.index);
        Ok(())
    }
}

impl Encode for TxInput {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtocolError> {
        self.previous_output.encode(out)?;
        put_bytes(out, &self.unlocking_data)
    }
}

impl Encode for TxOutput {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtocolError> {
        put_u64_le(out, self.value);
        put_bytes(out, &self.spending_condition)
    }
}

impl Encode for Transaction {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtocolError> {
        put_u32_le(out, self.version);
        put_u32_le(
            out,
            u32::try_from(self.inputs.len()).map_err(|_| ProtocolError::LengthOverflow)?,
        );
        for input in &self.inputs {
            input.encode(out)?;
        }
        put_u32_le(
            out,
            u32::try_from(self.outputs.len()).map_err(|_| ProtocolError::LengthOverflow)?,
        );
        for output in &self.outputs {
            output.encode(out)?;
        }
        put_u64_le(out, self.fee);
        Ok(())
    }
}

impl Transaction {
    /// Coinbase transactions have no inputs and are created by block production.
    /// This is provisional until a dedicated transaction kind is frozen.
    pub fn is_coinbase(&self) -> bool {
        self.inputs.is_empty()
    }

    pub fn encode_to_vec(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut out = Vec::new();
        self.encode(&mut out)?;
        Ok(out)
    }

    /// Compute the transaction identifier from its canonical serialization.
    pub fn txid(&self) -> Result<Hash32, ProtocolError> {
        Ok(sha256(&self.encode_to_vec()?))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        Self::decode_bounded(bytes, &crate::params::ConsensusParams::devnet())
    }

    pub fn decode_bounded(
        bytes: &[u8],
        params: &crate::params::ConsensusParams,
    ) -> Result<Self, ProtocolError> {
        if bytes.len() > params.max_tx_bytes {
            return Err(ProtocolError::LimitExceeded);
        }
        let mut reader = Reader::new(bytes);
        let version = reader.read_u32_le()?;
        if version != params.tx_version {
            return Err(ProtocolError::UnsupportedVersion);
        }
        let input_count = reader.read_u32_le()? as usize;
        if input_count > params.max_inputs_per_tx {
            return Err(ProtocolError::LimitExceeded);
        }
        // Prevent OOM: capacity capped by validated count (<=256).
        let mut inputs = Vec::with_capacity(input_count);
        for _ in 0..input_count {
            let txid = Hash32(reader.read_array()?);
            let index = reader.read_u32_le()?;
            let raw = read_bytes_u32(&mut reader)?;
            if raw.len() > params.max_unlocking_bytes {
                return Err(ProtocolError::LimitExceeded);
            }
            inputs.push(TxInput {
                previous_output: OutPoint { txid, index },
                unlocking_data: raw.to_vec(),
            });
        }
        let output_count = reader.read_u32_le()? as usize;
        if output_count > params.max_outputs_per_tx {
            return Err(ProtocolError::LimitExceeded);
        }
        let mut outputs = Vec::with_capacity(output_count);
        for _ in 0..output_count {
            let value = reader.read_u64_le()?;
            if value > params.max_money {
                return Err(ProtocolError::LimitExceeded);
            }
            let raw = read_bytes_u32(&mut reader)?;
            if raw.len() > params.max_spending_condition_bytes {
                return Err(ProtocolError::LimitExceeded);
            }
            outputs.push(TxOutput {
                value,
                spending_condition: raw.to_vec(),
            });
        }
        let fee = reader.read_u64_le()?;
        if fee > params.max_money {
            return Err(ProtocolError::LimitExceeded);
        }
        reader.finish()?;
        Ok(Self {
            version,
            inputs,
            outputs,
            fee,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_encoding_is_deterministic() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint {
                    txid: Hash32([7u8; 32]),
                    index: 3,
                },
                unlocking_data: b"sig".to_vec(),
            }],
            outputs: vec![TxOutput {
                value: 123,
                spending_condition: vec![0u8; 32],
            }],
            fee: 2,
        };
        let a = tx.encode_to_vec().unwrap();
        let b = tx.encode_to_vec().unwrap();
        assert_eq!(a, b);
        assert_eq!(Transaction::decode(&a).unwrap(), tx);
        assert_eq!(tx.txid().unwrap(), sha256(&a));
        assert_eq!(&a[..4], &1u32.to_le_bytes());
    }

    #[test]
    fn oversized_counts_and_versions_are_rejected() {
        use crate::params::ConsensusParams;
        let params = ConsensusParams::devnet();
        // Wrong version.
        let tx = Transaction {
            version: 999,
            inputs: vec![],
            outputs: vec![],
            fee: 0,
        };
        let enc = tx.encode_to_vec().unwrap();
        assert_eq!(
            Transaction::decode_bounded(&enc, &params),
            Err(ProtocolError::UnsupportedVersion)
        );
        // Too many inputs claimed on the wire.
        let mut raw = Vec::new();
        crate::codec::put_u32_le(&mut raw, 1);
        crate::codec::put_u32_le(&mut raw, 1000);
        assert_eq!(
            Transaction::decode_bounded(&raw, &params),
            Err(ProtocolError::LimitExceeded)
        );
        // Trailing bytes.
        let mut enc = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                value: 1,
                spending_condition: vec![0u8; 32],
            }],
            fee: 0,
        }
        .encode_to_vec()
        .unwrap();
        enc.push(0);
        assert_eq!(
            Transaction::decode_bounded(&enc, &params),
            Err(ProtocolError::TrailingBytes)
        );
    }
}
