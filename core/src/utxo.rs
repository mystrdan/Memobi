//! UTXO validation skeleton.
//!
//! Signature verification and final script/spending-condition semantics are
//! intentionally deferred until the cryptographic design is frozen.

use std::collections::HashMap;

use crate::{OutPoint, Transaction};

pub type UtxoSet = HashMap<OutPoint, UtxoEntry>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    pub value: u64,
    pub spending_condition: Vec<u8>,
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
    InvalidFee,
}

pub fn validate_transaction(tx: &Transaction, utxos: &UtxoSet) -> Result<(), ValidationError> {
    if tx.inputs.is_empty() {
        return Err(ValidationError::EmptyInputs);
    }
    if tx.outputs.is_empty() {
        return Err(ValidationError::EmptyOutputs);
    }

    let mut seen = std::collections::HashSet::with_capacity(tx.inputs.len());
    let mut input_total = 0u64;

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

        input_total = input_total
            .checked_add(entry.value)
            .ok_or(ValidationError::InputValueOverflow)?;
    }

    let mut output_total = 0u64;
    for (index, output) in tx.outputs.iter().enumerate() {
        if output.value == 0 {
            return Err(ValidationError::ZeroOutput { index });
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
    use crate::{hash::Hash32, OutPoint, TxInput, TxOutput};

    fn outpoint(index: u32) -> OutPoint {
        OutPoint {
            txid: Hash32([index as u8; 32]),
            index,
        }
    }

    #[test]
    fn valid_transaction_conserves_value() {
        let op = outpoint(0);
        let mut utxos = UtxoSet::new();
        utxos.insert(
            op,
            UtxoEntry {
                value: 100,
                spending_condition: b"condition".to_vec(),
            },
        );

        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: op,
                unlocking_data: b"placeholder".to_vec(),
            }],
            outputs: vec![TxOutput {
                value: 90,
                spending_condition: b"condition".to_vec(),
            }],
            fee: 10,
        };

        assert_eq!(validate_transaction(&tx, &utxos), Ok(()));
    }

    #[test]
    fn duplicate_inputs_are_rejected() {
        let op = outpoint(1);
        let mut utxos = UtxoSet::new();
        utxos.insert(
            op,
            UtxoEntry {
                value: 100,
                spending_condition: Vec::new(),
            },
        );

        let input = TxInput {
            previous_output: op,
            unlocking_data: Vec::new(),
        };
        let tx = Transaction {
            version: 1,
            inputs: vec![input.clone(), input],
            outputs: vec![TxOutput {
                value: 190,
                spending_condition: Vec::new(),
            }],
            fee: 10,
        };

        assert!(matches!(
            validate_transaction(&tx, &utxos),
            Err(ValidationError::DuplicateInput { .. })
        ));
    }

    #[test]
    fn missing_inputs_are_rejected() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: outpoint(2),
                unlocking_data: Vec::new(),
            }],
            outputs: vec![TxOutput {
                value: 1,
                spending_condition: Vec::new(),
            }],
            fee: 0,
        };

        assert!(matches!(
            validate_transaction(&tx, &UtxoSet::new()),
            Err(ValidationError::MissingInput { .. })
        ));
    }
}
