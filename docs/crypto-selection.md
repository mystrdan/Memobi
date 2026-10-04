# Cryptography Selection

## Status

Research decision pending. No signature primitive is consensus-frozen.

## Candidate set

The initial comparison is deliberately small:

- **Ed25519**
- **secp256k1**

No custom cryptography will be designed for Memobi.

## Evaluation criteria

Compare candidates on:

1. Security maturity and review history.
2. Quality of maintained Rust implementations.
3. Android/ARM64 availability.
4. Key generation and signing performance on phone-class CPUs.
5. Verification performance during block/transaction validation.
6. Signature and public-key sizes.
7. Straightforward address encoding.
8. Cross-language interoperability.
9. Long-term maintenance and auditability.

## Protocol principle

Wallet cryptography must remain separate from PoARM. Changing a wallet signature primitive must not alter the mining algorithm.

## Required test matrix

Before freezing the primitive, generate deterministic test vectors covering:

- private-key generation/import;
- public-key derivation;
- signing;
- signature verification;
- invalid signature rejection;
- altered message rejection;
- malformed key/signature rejection;
- address payload encoding;
- round-trip address decoding.

The final choice should be based on measured mobile performance plus implementation/security considerations, not popularity alone.
