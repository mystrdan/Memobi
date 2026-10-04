# Memobi Protocol Specification

## Status

This is the current design direction, not a mainnet consensus freeze.

## Identity

- Chain: Memobi
- Native asset: MEMO
- Consensus: PoARM (Proof of ARM)
- Transaction model: UTXO
- Smart contracts: none
- EVM compatibility: none
- Core implementation: Rust

## Core principle

Memobi is intended to be a permissionless Proof-of-Work network where smartphone-class CPUs are economically relevant mining hardware.

The protocol must not depend on a centralized mining service, account provider, or cloud backend.

## Initial network assumptions

The first engineering target is a devnet with a short block interval suitable for rapid experimentation. A 10-second target is a starting hypothesis, not a final consensus parameter.

## Addressing

The wallet is expected to use Bech32/Bech32m-style addresses with an mb1... mainnet prefix. Devnet and testnet prefixes will be distinct.

## Transactions

Memobi will initially use a UTXO transaction model. Inputs reference previous outputs, outputs contain value and spending conditions, signatures authorize spending, and fees are paid from transaction inputs.

No gas model is required because Memobi will not execute smart contracts.

## Blocks

A block will eventually contain at minimum:

- protocol version
- previous block hash
- block height
- timestamp
- difficulty target
- PoARM proof and nonce data
- transaction commitment
- transactions
- miner reward

Exact serialization is intentionally not frozen yet.

## Validation

A node must independently validate block structure, previous-block linkage, proof-of-work, difficulty, transaction serialization, signatures, UTXO availability, value conservation, fees, block subsidy, coinbase maturity, and chain selection.

## Design rule

Consensus-critical behavior must be deterministic across supported platforms. Mobile-specific safety features such as thermal throttling or battery limits belong in the client and must not change consensus.
