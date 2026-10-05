//! Wallet: deterministic key derivation, addresses, and transaction signing.
//!
//! Keys derive from a 32-byte seed via domain-separated hashing; addresses
//! are bech32 over `sha256(pubkey)[..20]` with the network HRP from
//! [`ConsensusParams`]. Derivation is provisional pending the wallet key
//! selection gate (`docs/wallet-key-selection.md`); no custom cryptography.

use zeroize::Zeroize;

use crate::{
    ProtocolError,
    crypto::{SecretKey, encode_address},
    hash::sha256,
    params::ConsensusParams,
    transaction::Transaction,
};

const KEY_DOMAIN: &[u8] = b"MEMOBI-KEY-V1";

/// Deterministic wallet over a 32-byte seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallet {
    seed: [u8; 32],
    next_index: u32,
}

impl Wallet {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            seed,
            next_index: 0,
        }
    }

    /// Deterministic child key for `index` (domain-separated SHA-256 over
    /// seed and index; replace with the gated KDF before mainnet).
    pub fn derive_key(&self, index: u32) -> SecretKey {
        let mut msg = Vec::with_capacity(KEY_DOMAIN.len() + 32 + 4);
        msg.extend_from_slice(KEY_DOMAIN);
        msg.extend_from_slice(&self.seed);
        msg.extend_from_slice(&index.to_le_bytes());
        let mut digest = sha256(&msg);
        let mut bytes = digest.0;
        let key = SecretKey::from_bytes(&bytes);
        bytes.zeroize();
        digest.0.zeroize();
        key
    }

    /// Reserve the next index and return it with its key.
    pub fn new_key(&mut self) -> (u32, SecretKey) {
        let index = self.next_index;
        self.next_index = self.next_index.wrapping_add(1);
        (index, self.derive_key(index))
    }

    /// Next index that [`Wallet::new_key`] would hand out.
    pub fn peek_next_index(&self) -> u32 {
        self.next_index
    }

    /// Bech32 address for key `index` under the network HRP.
    pub fn address(&self, index: u32, params: &ConsensusParams) -> String {
        address_from_pubkey(&self.derive_key(index).public_key(), params)
    }

    /// Spending condition to place in `TxOutput`s locked to key `index`
    /// (raw 32-byte pubkey for spending-condition v1).
    pub fn spending_condition(&self, index: u32) -> Vec<u8> {
        self.derive_key(index).public_key().to_vec()
    }

    /// Sign `input_index` of `tx`, writing `pubkey||signature` unlocking data.
    pub fn sign_input(
        &self,
        index: u32,
        tx: &mut Transaction,
        input_index: usize,
        params: &ConsensusParams,
    ) -> Result<(), ProtocolError> {
        let key = self.derive_key(index);
        let unlocking = crate::crypto::authorize_input(tx, input_index, &key, params)?;
        let input = tx
            .inputs
            .get_mut(input_index)
            .ok_or(ProtocolError::UnexpectedEof)?;
        input.unlocking_data = unlocking;
        Ok(())
    }
}

/// Bech32 address for a 32-byte pubkey under the network HRP.
pub fn address_from_pubkey(pubkey: &[u8; 32], params: &ConsensusParams) -> String {
    let mut digest = sha256(pubkey);
    let mut program = [0u8; 20];
    program.copy_from_slice(&digest.0[..20]);
    digest.0.zeroize();
    encode_address(params.address_hrp, &program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hash32, OutPoint, TxInput, TxOutput, crypto::verify_inputs};

    #[test]
    fn derivation_is_deterministic_and_indexed() {
        let wallet = Wallet::from_seed([7u8; 32]);
        let same = Wallet::from_seed([7u8; 32]);
        assert_eq!(wallet.derive_key(0).public_key(), same.derive_key(0).public_key());
        assert_ne!(wallet.derive_key(0).public_key(), wallet.derive_key(1).public_key());
        assert_ne!(
            wallet.derive_key(0).public_key(),
            Wallet::from_seed([8u8; 32]).derive_key(0).public_key()
        );
    }

    #[test]
    fn address_uses_network_hrp_and_round_trips() {
        let devnet = ConsensusParams::devnet();
        let testnet = ConsensusParams::testnet();
        let wallet = Wallet::from_seed([9u8; 32]);
        let dev = wallet.address(0, &devnet);
        let tst = wallet.address(0, &testnet);
        assert!(dev.starts_with(&format!("{}1", devnet.address_hrp)));
        assert!(tst.starts_with(&format!("{}1", testnet.address_hrp)));
        assert_ne!(dev, tst);
        let (hrp, _) = crate::crypto::decode_address(&dev).unwrap();
        assert_eq!(hrp, devnet.address_hrp);
    }

    #[test]
    fn sign_input_produces_verifiable_unlocking_data() {
        let params = ConsensusParams::devnet();
        let wallet = Wallet::from_seed([3u8; 32]);
        let condition = wallet.spending_condition(0);
        let mut tx = Transaction {
            version: params.tx_version,
            inputs: vec![TxInput {
                previous_output: OutPoint {
                    txid: Hash32([4u8; 32]),
                    index: 0,
                },
                unlocking_data: Vec::new(),
            }],
            outputs: vec![TxOutput {
                value: 100,
                spending_condition: condition.clone(),
            }],
            fee: 1,
        };
        wallet.sign_input(0, &mut tx, 0, &params).unwrap();
        verify_inputs(&tx, std::slice::from_ref(&condition), &params).unwrap();
    }

    #[test]
    fn new_key_advances_indices() {
        let mut wallet = Wallet::from_seed([1u8; 32]);
        assert_eq!(wallet.peek_next_index(), 0);
        let (i0, k0) = wallet.new_key();
        let (i1, k1) = wallet.new_key();
        assert_eq!((i0, i1), (0, 1));
        assert_eq!(k0.public_key(), wallet.derive_key(0).public_key());
        assert_eq!(k1.public_key(), wallet.derive_key(1).public_key());
    }
}