# Memobi Genesis

Status: **research/devnet draft**.

Genesis is consensus-critical. No mainnet genesis values are frozen yet.

## Required properties

The genesis definition must deterministically specify:

- chain identifier;
- network identifier;
- genesis timestamp;
- genesis block header;
- initial transaction set, if any;
- initial UTXO state;
- initial PoARM version;
- initial target;
- block height;
- expected genesis block identifier.

## No hidden allocation

The current economic direction is **no premine**. If genesis contains any spendable MEMO, that must be explicitly justified and represented by deterministic consensus data.

The preferred path is for initial MEMO issuance to begin through the finalized block-reward schedule rather than an opaque genesis allocation.

## Devnet vs mainnet

Devnet may use clearly marked parameters and genesis values. Devnet identifiers must not be accepted as mainnet chain identity.

## Freeze gate

Genesis should only be frozen after:

1. block serialization is frozen;
2. PoARM version/target rules are frozen;
3. difficulty rules are frozen;
4. reward/emission rules are frozen;
5. transaction/address formats are frozen;
6. deterministic vectors reproduce the same genesis ID.
