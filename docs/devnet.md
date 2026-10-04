# Memobi Devnet

Status: **development scaffold**. This document describes the first runnable local-network milestone; it is not a mainnet specification.

## Goal

The first devnet should prove the complete local loop:

1. create or load a local chain state;
2. construct a candidate block;
3. run PoARM work;
4. check the proof against a target;
5. apply the block deterministically;
6. expose enough state for a future CLI/node and Flutter client.

## Deliberate limits

The first devnet does not require:

- finalized wallet cryptography;
- finalized address encoding;
- production P2P;
- finalized emission economics;
- finalized difficulty;
- smart contracts.

Those remain separate protocol decisions.

## Local chain

A devnet may use an in-memory chain initially. Persistence can follow after the state transition rules are stable.

The chain state must never trust a miner's local balance or block claim. Every candidate block is validated by the same Rust core.

## Mining loop

Conceptually:

tip -> candidate header -> PoARM(seed, nonce, epoch) -> target check -> block validation -> state transition -> new tip

The miner changes the nonce and repeats until the resulting PoARM value satisfies the current target.

## Success criteria

A devnet milestone is successful when two independent Rust processes can:

- start from the same genesis;
- derive the same chain identity;
- produce and validate compatible blocks;
- reject invalid previous-block links;
- reject invalid transactions;
- observe the same tip after valid blocks are applied.

## Next implementation

The next code should add a minimal Coinbase/reward distinction and a deterministic genesis constructor before wiring a full mining loop.
