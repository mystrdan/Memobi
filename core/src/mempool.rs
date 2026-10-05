//! Mempool: consensus validation separated from relay policy.
//!
//! Consensus (`validate_transaction` + signatures) must pass before any
//! policy (fees, size, double-spend-in-pool, expiry) is applied.

use std::collections::{BTreeMap, HashMap};

use crate::{Hash32, OutPoint, Transaction, params::ConsensusParams, utxo::ValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MempoolError {
    Consensus(ValidationError),
    InvalidSignature,
    AlreadyKnown,
    DoubleSpendInPool,
    TooManyTransactions,
    PoolBytesExceeded,
    TxTooLarge,
    FeeTooLow { fee: u64, min_fee: u64 },
}

#[derive(Debug, Clone)]
struct Entry {
    tx: Transaction,
    bytes: usize,
    added_at: u64,
    fee: u64,
}

#[derive(Debug, Default)]
pub struct Mempool {
    entries: HashMap<Hash32, Entry>,
    spent: HashMap<OutPoint, Hash32>,
    by_fee: BTreeMap<(u64, Hash32), Hash32>,
    total_bytes: usize,
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
    pub fn contains(&self, txid: &Hash32) -> bool {
        self.entries.contains_key(txid)
    }
    fn remove(&mut self, txid: &Hash32) {
        if let Some(e) = self.entries.remove(txid) {
            self.total_bytes = self.total_bytes.saturating_sub(e.bytes);
            self.by_fee.remove(&(e.fee, *txid));
            for input in &e.tx.inputs {
                if self.spent.get(&input.previous_output) == Some(txid) {
                    self.spent.remove(&input.previous_output);
                }
            }
        }
    }
    pub fn insert(
        &mut self,
        tx: Transaction,
        utxos: &crate::utxo::UtxoSet,
        params: &ConsensusParams,
        tip_height: u64,
        now_secs: u64,
        min_fee: u64,
    ) -> Result<Hash32, MempoolError> {
        if tx.is_coinbase() {
            return Err(MempoolError::Consensus(ValidationError::EmptyInputs));
        }
        let bytes = tx.encode_to_vec().map_err(|_| MempoolError::TxTooLarge)?;
        if bytes.len() > params.max_tx_bytes {
            return Err(MempoolError::TxTooLarge);
        }
        if tx.fee < min_fee {
            return Err(MempoolError::FeeTooLow { fee: tx.fee, min_fee });
        }
        crate::utxo::validate_transaction(&tx, utxos, tip_height + 1, params)
            .map_err(MempoolError::Consensus)?;
        let conds: Vec<Vec<u8>> = tx
            .inputs
            .iter()
            .map(|i| {
                utxos
                    .get(&i.previous_output)
                    .map(|e| e.spending_condition.clone())
                    .unwrap_or_default()
            })
            .collect();
        crate::crypto::verify_inputs(&tx, &conds, params)
            .map_err(|_| MempoolError::InvalidSignature)?;
        let txid = tx.txid().map_err(|_| MempoolError::TxTooLarge)?;
        if self.entries.contains_key(&txid) {
            return Err(MempoolError::AlreadyKnown);
        }
        for input in &tx.inputs {
            if self.spent.contains_key(&input.previous_output) {
                return Err(MempoolError::DoubleSpendInPool);
            }
        }
        if self.entries.len() >= params.mempool_max_txs {
            return Err(MempoolError::TooManyTransactions);
        }
        if self.total_bytes.saturating_add(bytes.len()) > params.mempool_max_bytes {
            return Err(MempoolError::PoolBytesExceeded);
        }
        self.by_fee.insert((tx.fee, txid), txid);
        for input in &tx.inputs {
            self.spent.insert(input.previous_output, txid);
        }
        self.total_bytes += bytes.len();
        self.entries.insert(
            txid,
            Entry { tx, bytes: bytes.len(), added_at: now_secs, fee: 0 },
        );
        if let Some(e) = self.entries.get_mut(&txid) {
            e.fee = e.tx.fee;
            let fee = e.fee;
            self.by_fee.remove(&(0, txid));
            self.by_fee.insert((fee, txid), txid);
        }
        Ok(txid)
    }
    pub fn remove_confirmed(&mut self, txs: &[Transaction]) {
        for tx in txs {
            if let Ok(txid) = tx.txid() {
                self.remove(&txid);
            }
        }
    }
    pub fn expire(&mut self, now_secs: u64, params: &ConsensusParams) {
        let stale: Vec<Hash32> = self
            .entries
            .iter()
            .filter(|(_, e)| now_secs.saturating_sub(e.added_at) > params.mempool_max_age_secs)
            .map(|(k, _)| *k)
            .collect();
        for k in stale {
            self.remove(&k);
        }
    }
    pub fn candidates(&self, limit: usize) -> Vec<Transaction> {
        let mut v: Vec<(u64, Hash32, Transaction)> = self
            .entries
            .iter()
            .map(|(id, e)| (e.fee, *id, e.tx.clone()))
            .collect();
        v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.0.cmp(&b.1.0)));
        v.into_iter().take(limit).map(|(_, _, tx)| tx).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{SecretKey, authorize_input};
    use crate::utxo::{UtxoEntry, UtxoSet};

    fn funded(_params: &ConsensusParams) -> (UtxoSet, SecretKey, OutPoint) {
        let sk = SecretKey::from_bytes(&[11u8; 32]);
        let pk = sk.public_key();
        let op = OutPoint {
            txid: Hash32([9u8; 32]),
            index: 0,
        };
        let mut utxos = UtxoSet::new();
        utxos.insert(
            op,
            UtxoEntry {
                value: 1000,
                spending_condition: pk.to_vec(),
                height: 0,
                is_coinbase: false,
            },
        );
        (utxos, sk, op)
    }

    fn spend(
        op: OutPoint,
        value: u64,
        sk: &SecretKey,
        params: &ConsensusParams,
    ) -> Transaction {
        let mut tx = Transaction {
            version: params.tx_version,
            inputs: vec![crate::TxInput {
                previous_output: op,
                unlocking_data: Vec::new(),
            }],
            outputs: vec![crate::TxOutput {
                value,
                spending_condition: sk.public_key().to_vec(),
            }],
            fee: 1000 - value,
        };
        tx.inputs[0].unlocking_data = authorize_input(&tx, 0, sk, params).unwrap();
        tx
    }

    #[test]
    fn accepts_and_rejects_double_spend_in_pool() {
        let params = ConsensusParams::devnet();
        let (utxos, sk, op) = funded(&params);
        let mut pool = Mempool::new();
        let tx1 = spend(op, 900, &sk, &params);
        pool.insert(tx1, &utxos, &params, 0, 100, 0).unwrap();
        let tx2 = spend(op, 800, &sk, &params);
        assert_eq!(
            pool.insert(tx2, &utxos, &params, 0, 100, 0),
            Err(MempoolError::DoubleSpendInPool)
        );
    }

    #[test]
    fn fee_policy_and_expiry() {
        let params = ConsensusParams::devnet();
        let (utxos, sk, op) = funded(&params);
        let mut pool = Mempool::new();
        let tx = spend(op, 999, &sk, &params);
        assert!(matches!(
            pool.insert(tx.clone(), &utxos, &params, 0, 0, 100),
            Err(MempoolError::FeeTooLow { .. })
        ));
        pool.insert(tx, &utxos, &params, 0, 0, 0).unwrap();
        pool.expire(params.mempool_max_age_secs + 1, &params);
        assert!(pool.is_empty());
    }
}
