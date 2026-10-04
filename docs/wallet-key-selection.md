# Wallet key selection gate

Memobi needs a native wallet, but the signing primitive is not consensus-frozen yet.

## Candidates

- Ed25519
- secp256k1

## Selection gate

The final choice should be based on:

1. Security maturity and review history.
2. Maintained Rust implementations.
3. Android ARM64 availability.
4. Key generation and signing performance on phone-class CPUs.
5. Signature verification performance.
6. Public-key and signature size.
7. Address encoding simplicity.
8. Interoperability and tooling.
9. Long-term maintenance and auditability.

## Required test vectors

Before freezing the choice, publish deterministic vectors for:

- private-key import/export
- public-key derivation
- signing
- signature verification
- altered-message rejection
- malformed key rejection
- malformed signature rejection
- address payload construction
- address encode/decode round trip

## Boundary

Wallet cryptography must remain separate from PoARM. Mining identity does not need to become a consensus identity system.

No custom cryptography should be introduced.

The final primitive will be selected after implementation-quality review and mobile measurements rather than convenience alone.
