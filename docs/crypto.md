# Memobi Cryptography and Addressing

## Status

Design study. No cryptographic primitive is frozen by this document.

## Requirements

Memobi needs:

- secure private-key generation
- deterministic signing and verification
- compact transaction authorization
- address encoding with human-readable prefixes
- explicit domain separation where signatures are used
- byte-for-byte test vectors
- implementations available for Rust and Android

## Candidate signing systems

The first implementation should benchmark and review modern signature choices before freezing one. Candidates include Ed25519 and secp256k1.

The choice must consider:

- security maturity
- audit history
- implementation quality
- mobile performance
- key/address interoperability
- signature size
- availability in the Rust ecosystem

## Address direction

The current direction is a Bech32/Bech32m-style human-readable address beginning with mb1 on mainnet.

Testnet and devnet must use distinct human-readable prefixes.

The address format must commit to a versioned payload so future formats can coexist without ambiguity.

## Rule

Do not invent a custom cryptographic primitive. Use established, reviewed primitives and keep consensus encoding independent from wallet UI.
