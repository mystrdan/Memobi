//! UTXO validation: value conservation, maturity, and Ed25519 authorization.

use std::collections::BTreeMap;

use crate::{Transaction, params::ConsensusParams, transaction::OutPoint};

pub type UtxoSet = BTreeMap<OutPoint, UtxoEntry>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    pub value: u64,
    pub spending_condition: Vec<u8>,
    pub height: u64,
    pub is_coinbase: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyInputs,
    EmptyOutputs,
    ZeroOutput { index: usize },
    DuplicateInput { outpoint: OutPoint },
    MissingInput { outpoint: OutPoint },
    InputValueOverflow,
    OutputValueOverflow,
    FeeValueOverflow,
    ValueMismatch { inputs: u64, outputs_and_fee: u64 },
    TooManyInputs,
    TooManyOutputs,
    OutputValueExceedsLimit,
    FeeExceedsLimit,
    ImmatureCoinbase { outpoint: OutPoint },
    InvalidPublicKey,
    InvalidSignature,
    UnsupportedVersion,
}

pub fn validate_transaction(
    tx: &Transaction,
    utxos: &UtxoSet,
    spend_height: u64,
    params: &ConsensusParams,
) -> Result<(), ValidationError> {
    if tx.version != params.tx_version {
        return Err(ValidationError::UnsupportedVersion);
    }
    if tx.inputs.is_empty() {
        return Err(ValidationError::EmptyInputs);
    }
    if tx.outputs.is_empty() {
        return Err(ValidationError::EmptyOutputs);
    }
    if tx.inputs.len() > params.max_inputs_per_tx {
        return Err(ValidationError::TooManyInputs);
    }
    if tx.outputs.len() > params.max_outputs_per_tx {
        return Err(ValidationError::TooManyOutputs);
    }
    if tx.fee > params.max_money {
        return Err(ValidationError::FeeExceedsLimit);
    }

    let mut seen = std::collections::HashSet::with_capacity(tx.inputs.len());
    let mut input_total = 0u64;
    let mut conditions: Vec<Vec<u8>> = Vec::with_capacity(tx.inputs.len());
    for input in &tx.inputs {
        if !seen.insert(input.previous_output) {
            return Err(ValidationError::DuplicateInput {
                outpoint: input.previous_output,
            });
        }
        let entry = utxos
            .get(&input.previous_output)
            .ok_or(ValidationError::MissingInput {
                outpoint: input.previous_output,
            })?;
        if entry.is_coinbase {
            let mature_at = entry.height.saturating_add(params.coinbase_maturity);
            if spend_height < mature_at {
                return Err(ValidationError::ImmatureCoinbase {
                    outpoint: input.previous_output,
                });
            }
        }
        if entry.spending_condition.len() != crate::crypto::PUBKEY_LEN {
            return Err(ValidationError::InvalidPublicKey);
        }
        conditions.push(entry.spending_condition.clone());
        input_total = input_total
            .checked_add(entry.value)
            .ok_or(ValidationError::InputValueOverflow)?;
        if input_total > params.max_money {
            return Err(ValidationError::InputValueOverflow);
        }
    }

    // Signatures: consensus-critical (prevents arbitrary spends).
    crate::crypto::verify_inputs(tx, &conditions, params)
        .map_err(|_| ValidationError::InvalidSignature)?;

    let mut output_total = 0u64;
    for (index, output) in tx.outputs.iter().enumerate() {
        if output.value == 0 {
            return Err(ValidationError::ZeroOutput { index });
        }
        if output.value > params.max_money {
            return Err(ValidationError::OutputValueExceedsLimit);
        }
        if output.spending_condition.len() != crate::crypto::PUBKEY_LEN {
            return Err(ValidationError::InvalidPublicKey);
        }
        output_total = output_total
            .checked_add(output.value)
            .ok_or(ValidationError::OutputValueOverflow)?;
    }
    let required = output_total
        .checked_add(tx.fee)
        .ok_or(ValidationError::FeeValueOverflow)?;
    if input_total != required {
        return Err(ValidationError::ValueMismatch {
            inputs: input_total,
            outputs_and_fee: required,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TxInput, TxOutput, hash::Hash32, params::ConsensusParams};

    fn outpoint(index: u32) -> OutPoint {
        OutPoint {
            txid: Hash32([index as u8; 32]),
            index,
        }
    }

    fn keyed_utxo(op: OutPoint, value: u64, pk: [u8; 32]) -> UtxoSet {
        let mut utxos = UtxoSet::new();
        utxos.insert(
            op,
            UtxoEntry {
                value,
                spending_condition: pk.to_vec(),
                height: 0,
                is_coinbase: false,
            },
        );
        utxos
    }

    fn signed_tx(
        op: OutPoint,
        value: u64,
        fee: u64,
        sk: &crate::crypto::SecretKey,
        params: &ConsensusParams,
    ) -> Transaction {
        let pk = sk.public_key();
        let mut tx = Transaction {
            version: params.tx_version,
            inputs: vec![TxInput {
                previous_output: op,
                unlocking_data: Vec::new(),
            }],
            outputs: vec![TxOutput {
                value,
                spending_condition: pk.to_vec(),
            }],
            fee,
        };
        tx.inputs[0].unlocking_data =
            crate::crypto::authorize_input(&tx, 0, sk, params).unwrap();
        tx
    }

    #[test]
    fn valid_transaction_conserves_value() {
        let params = ConsensusParams::devnet();
        let sk = crate::crypto::SecretKey::from_bytes(&[7u8; 32]);
        let op = outpoint(0);
        let utxos = keyed_utxo(op, 100, sk.public_key());
        let tx = signed_tx(op, 90, 10, &sk, &params);
        assert_eq!(validate_transaction(&tx, &utxos, 0, &params), Ok(()));
    }

    #[test]
    fn duplicate_inputs_are_rejected() {
        let params = ConsensusParams::devnet();
        let sk = crate::crypto::SecretKey::from_bytes(&[7u8; 32]);
        let op = outpoint(1);
        let utxos = keyed_utxo(op, 100, sk.public_key());
        let mut tx = signed_tx(op, 90, 10, &sk, &params);
        tx.inputs.push(tx.inputs[0].clone());
        assert!(matches!(
            validate_transaction(&tx, &utxos, 0, &params),
            Err(ValidationError::DuplicateInput { .. })
        ));
    }

    #[test]
    fn missing_inputs_are_rejected() {
        let params = ConsensusParams::devnet();
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: outpoint(2),
                unlocking_data: vec![0u8; 96],
            }],
            outputs: vec![TxOutput {
                value: 1,
                spending_condition: vec![0u8; 32],
            }],
            fee: 0,
        };
        assert!(matches!(
            validate_transaction(&tx, &UtxoSet::new(), 0, &params),
            Err(ValidationError::MissingInput { .. })
        ));
    }

    #[test]
    fn unsigned_spend_is_rejected() {
        let params = ConsensusParams::devnet();
        let sk = crate::crypto::SecretKey::from_bytes(&[7u8; 32]);
        let op = outpoint(3);
        let utxos = keyed_utxo(op, 100, sk.public_key());
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: op,
                unlocking_data: vec![0u8; 96],
            }],
            outputs: vec![TxOutput {
                value: 90,
                spending_condition: sk.public_key().to_vec(),
            }],
            fee: 10,
        };
        assert_eq!(
            validate_transaction(&tx, &utxos, 0, &params),
            Err(ValidationError::InvalidSignature)
        );
    }

    #[test]
    fn immature_coinbase_is_rejected() {
        let params = ConsensusParams::devnet();
        let sk = crate::crypto::SecretKey::from_bytes(&[8u8; 32]);
        let op = outpoint(4);
        let mut utxos = UtxoSet::new();
        utxos.insert(
            op,
            UtxoEntry {
                value: 50,
                spending_condition: sk.public_key().to_vec(),
                height: 100,
                is_coinbase: true,
            },
        );
        let tx = signed_tx(op, 40, 10, &sk, &params);
        assert!(matches!(
            validate_transaction(&tx, &utxos, 100 + params.coinbase_maturity - 1, &params),
            Err(ValidationError::ImmatureCoinbase { .. })
        ));
        assert_eq!(
            validate_transaction(&tx, &utxos, 100 + params.coinbase_maturity, &params),
            Ok(())
        );
    }
}
