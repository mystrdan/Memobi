//! Ed25519 transaction cryptography (consensus-critical).
//! Spending condition v1 = 32-byte pubkey; unlocking v1 = pubkey||signature.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use zeroize::Zeroize;

use crate::{
    ProtocolError,
    codec::{put_bytes, put_u32_le, put_u64_le},
    params::ConsensusParams,
};

pub const PUBKEY_LEN: usize = 32;
pub const SIGNATURE_LEN: usize = 64;
pub const UNLOCKING_LEN: usize = PUBKEY_LEN + SIGNATURE_LEN;
pub const SIGHASH_DOMAIN: &[u8] = b"MEMOBI-SIGHASH-V1";

pub struct SecretKey(SigningKey);

impl SecretKey {
    pub fn generate() -> Self {
        use rand_core::{OsRng, RngCore};
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let key = SigningKey::from_bytes(&bytes);
        bytes.zeroize();
        Self(key)
    }
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(SigningKey::from_bytes(bytes))
    }
    pub fn public_key(&self) -> [u8; 32] {
        self.0.verifying_key().to_bytes()
    }
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.0.sign(msg).to_bytes()
    }
}

pub fn verify_signature(pubkey: &[u8; 32], msg: &[u8], sig: &[u8; 64]) -> bool {
    let Ok(vk) = VerifyingKey::from_bytes(pubkey) else {
        return false;
    };
    vk.verify(msg, &Signature::from_bytes(sig)).is_ok()
}

/// Canonical message signed for `input_index`. Unlocking data excluded.
pub fn sighash(
    tx: &crate::Transaction,
    input_index: usize,
    params: &ConsensusParams,
) -> Result<crate::Hash32, ProtocolError> {
    use crate::hash::sha256;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(SIGHASH_DOMAIN);
    put_u32_le(&mut bytes, params.chain_id);
    put_u32_le(&mut bytes, tx.version);
    put_u32_le(
        &mut bytes,
        u32::try_from(tx.inputs.len()).map_err(|_| ProtocolError::LengthOverflow)?,
    );
    for input in &tx.inputs {
        bytes.extend_from_slice(input.previous_output.txid.as_bytes());
        put_u32_le(&mut bytes, input.previous_output.index);
    }
    put_u32_le(
        &mut bytes,
        u32::try_from(tx.outputs.len()).map_err(|_| ProtocolError::LengthOverflow)?,
    );
    for output in &tx.outputs {
        put_u64_le(&mut bytes, output.value);
        put_bytes(&mut bytes, &output.spending_condition)?;
    }
    put_u64_le(&mut bytes, tx.fee);
    put_u64_le(
        &mut bytes,
        u64::try_from(input_index).map_err(|_| ProtocolError::LengthOverflow)?,
    );
    Ok(sha256(&bytes))
}

/// Verify all inputs against expected 32-byte pubkey conditions.
pub fn verify_inputs(
    tx: &crate::Transaction,
    conditions: &[Vec<u8>],
    params: &ConsensusParams,
) -> Result<(), ProtocolError> {
    if tx.is_coinbase() {
        return Ok(());
    }
    if conditions.len() != tx.inputs.len() {
        return Err(ProtocolError::InvalidSignature);
    }
    for (i, (input, cond)) in tx.inputs.iter().zip(conditions.iter()).enumerate() {
        if cond.len() != PUBKEY_LEN {
            return Err(ProtocolError::InvalidPublicKey);
        }
        if input.unlocking_data.len() != UNLOCKING_LEN {
            return Err(ProtocolError::InvalidSignature);
        }
        let mut pk = [0u8; 32];
        let mut sig = [0u8; 64];
        pk.copy_from_slice(&input.unlocking_data[..32]);
        sig.copy_from_slice(&input.unlocking_data[32..]);
        if pk != cond.as_slice() {
            return Err(ProtocolError::InvalidSignature);
        }
        let msg = sighash(tx, i, params)?;
        if !verify_signature(&pk, msg.as_bytes(), &sig) {
            return Err(ProtocolError::InvalidSignature);
        }
    }
    Ok(())
}

/// Build unlocking data for `input_index` using `secret`.
pub fn authorize_input(
    tx: &crate::Transaction,
    input_index: usize,
    secret: &SecretKey,
    params: &ConsensusParams,
) -> Result<Vec<u8>, ProtocolError> {
    let msg = sighash(tx, input_index, params)?;
    let pk = secret.public_key();
    let sig = secret.sign(msg.as_bytes());
    let mut out = Vec::with_capacity(UNLOCKING_LEN);
    out.extend_from_slice(&pk);
    out.extend_from_slice(&sig);
    Ok(out)
}

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

fn polymod(values: &[u8]) -> u32 {
    let mut chk: u32 = 1;
    for v in values {
        let b = chk >> 25;
        chk = ((chk & 0x1ff_ffff) << 5) ^ (*v as u32);
        if b & 1 != 0 {
            chk ^= 0x3b6a_57b2;
        }
        if b & 2 != 0 {
            chk ^= 0x2650_8e6d;
        }
        if b & 4 != 0 {
            chk ^= 0x1ea1_19fa;
        }
        if b & 8 != 0 {
            chk ^= 0x03d4_23dd;
        }
        if b & 16 != 0 {
            chk ^= 0x2a14_62e1;
        }
    }
    chk
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in hrp.bytes() {
        out.push(c >> 5);
    }
    out.push(0);
    for c in hrp.bytes() {
        out.push(c & 31);
    }
    out
}

fn convert_bits(data: &[u8], from: u32, to: u32, pad: bool) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut out = Vec::new();
    let maxv = (1u32 << to) - 1;
    for b in data {
        acc = (acc << from) | (*b as u32);
        bits += from;
        while bits >= to {
            bits -= to;
            out.push(((acc >> bits) & maxv) as u8);
        }
    }
    if pad {
        if bits > 0 {
            out.push(((acc << (to - bits)) & maxv) as u8);
        }
    } else if bits >= from || ((acc << (to - bits)) & maxv) != 0 {
        return None;
    }
    Some(out)
}

/// Encode 20-byte program as bech32 address with given HRP.
pub fn encode_address(hrp: &str, program: &[u8; 20]) -> String {
    let mut data = vec![0u8];
    data.extend(convert_bits(program, 8, 5, true).unwrap());
    let mut values = hrp_expand(hrp);
    values.extend(&data);
    values.extend([0u8; 6]);
    let pm = polymod(&values) ^ 1;
    for i in 0..6 {
        data.push(((pm >> (5 * (5 - i))) & 31) as u8);
    }
    let mut s = String::with_capacity(hrp.len() + 1 + data.len());
    s.push_str(hrp);
    s.push('1');
    for d in data {
        s.push(CHARSET[d as usize] as char);
    }
    s
}

/// Decode bech32 address; returns `(hrp, program20)`.
pub fn decode_address(addr: &str) -> Option<(String, [u8; 20])> {
    let addr = addr.to_lowercase();
    let pos = addr.rfind('1')?;
    let hrp = addr[..pos].to_string();
    if hrp.is_empty() || hrp.len() > 83 {
        return None;
    }
    let mut data = Vec::new();
    for c in addr[pos + 1..].bytes() {
        data.push(CHARSET.iter().position(|&x| x == c)? as u8);
    }
    if data.len() < 7 {
        return None;
    }
    let mut values = hrp_expand(&hrp);
    values.extend(&data);
    if polymod(&values) != 1 {
        return None;
    }
    let payload = &data[..data.len() - 6];
    if payload.is_empty() || payload[0] != 0 {
        return None;
    }
    let prog = convert_bits(&payload[1..], 5, 8, false)?;
    if prog.len() != 20 {
        return None;
    }
    let mut out = [0u8; 20];
    out.copy_from_slice(&prog);
    Some((hrp, out))
}

/// Address program: first 20 bytes of sha256(pubkey).
pub fn address_program(pubkey: &[u8; 32]) -> [u8; 20] {
    use crate::hash::sha256;
    let h = sha256(pubkey);
    let mut out = [0u8; 20];
    out.copy_from_slice(&h.as_bytes()[..20]);
    out
}

pub fn pubkey_address(pubkey: &[u8; 32], params: &ConsensusParams) -> String {
    encode_address(params.address_hrp, &address_program(pubkey))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::ConsensusParams;

    fn test_tx(pk: [u8; 32]) -> crate::Transaction {
        crate::Transaction {
            version: 1,
            inputs: vec![crate::TxInput {
                previous_output: crate::OutPoint {
                    txid: crate::Hash32([1u8; 32]),
                    index: 0,
                },
                unlocking_data: Vec::new(),
            }],
            outputs: vec![crate::TxOutput {
                value: 90,
                spending_condition: pk.to_vec(),
            }],
            fee: 10,
        }
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let params = ConsensusParams::devnet();
        let sk = SecretKey::from_bytes(&[7u8; 32]);
        let pk = sk.public_key();
        let tx = test_tx(pk);
        let unlocking = authorize_input(&tx, 0, &sk, &params).unwrap();
        assert_eq!(unlocking.len(), UNLOCKING_LEN);
        let mut signed = tx.clone();
        signed.inputs[0].unlocking_data = unlocking;
        verify_inputs(&signed, &[pk.to_vec()], &params).unwrap();
    }

    #[test]
    fn altered_message_is_rejected() {
        let params = ConsensusParams::devnet();
        let sk = SecretKey::from_bytes(&[9u8; 32]);
        let pk = sk.public_key();
        let tx = test_tx(pk);
        let unlocking = authorize_input(&tx, 0, &sk, &params).unwrap();
        let mut tampered = tx.clone();
        tampered.inputs[0].unlocking_data = unlocking;
        tampered.outputs[0].value = 91;
        assert!(verify_inputs(&tampered, &[pk.to_vec()], &params).is_err());
    }

    #[test]
    fn wrong_chain_id_is_rejected() {
        let a = ConsensusParams::devnet();
        let mut b = ConsensusParams::devnet();
        b.chain_id = 0xdead_beef;
        let sk = SecretKey::from_bytes(&[5u8; 32]);
        let pk = sk.public_key();
        let tx = test_tx(pk);
        let unlocking = authorize_input(&tx, 0, &sk, &a).unwrap();
        let mut signed = tx.clone();
        signed.inputs[0].unlocking_data = unlocking;
        assert!(verify_inputs(&signed, &[pk.to_vec()], &b).is_err());
    }

    #[test]
    fn address_round_trip_with_hrp() {
        let params = ConsensusParams::devnet();
        let pk = SecretKey::from_bytes(&[3u8; 32]).public_key();
        let addr = pubkey_address(&pk, &params);
        assert!(addr.starts_with("mbd1"));
        let (hrp, prog) = decode_address(&addr).unwrap();
        assert_eq!(hrp, "mbd");
        assert_eq!(prog, address_program(&pk));
        let testnet = ConsensusParams::testnet();
        let addr_t = pubkey_address(&pk, &testnet);
        assert!(addr_t.starts_with("mbt1"));
        assert_ne!(addr, addr_t);
    }

    #[test]
    fn malformed_inputs_rejected() {
        let params = ConsensusParams::devnet();
        let tx = crate::Transaction {
            version: 1,
            inputs: vec![crate::TxInput {
                previous_output: crate::OutPoint {
                    txid: crate::Hash32([1u8; 32]),
                    index: 0,
                },
                unlocking_data: vec![0u8; 10],
            }],
            outputs: vec![crate::TxOutput {
                value: 1,
                spending_condition: vec![0u8; 32],
            }],
            fee: 0,
        };
        assert!(verify_inputs(&tx, &[vec![0u8; 32]], &params).is_err());
    }

    #[test]
    fn bad_checksum_rejected() {
        let params = ConsensusParams::devnet();
        let pk = SecretKey::from_bytes(&[3u8; 32]).public_key();
        let mut addr = pubkey_address(&pk, &params);
        let last = addr.pop().unwrap();
        addr.push(if last == 'q' { 'p' } else { 'q' });
        assert!(decode_address(&addr).is_none());
    }
}
