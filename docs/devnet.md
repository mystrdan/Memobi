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

The repository now has a reusable deterministic multi-block producer in `poarm/src/devnet.rs`. It starts from the real devnet genesis, builds each block from the current chain tip, derives the consensus difficulty target, constructs the real coinbase, mines Candidate C, independently verifies the proof, validates the block, and applies it through `ChainState`.

The producer is deliberately configured with a tiny PoARM workload and deterministic timestamps so it is suitable for repeatable protocol tests rather than performance claims.

It is exercised by unit tests for:
- multi-block production;
- deterministic repeated runs;
- target derivation through the consensus difficulty path.

The command-line PoARM lab now runs this producer after the candidate benchmark and reports the final height, tip, and cumulative work.

The next devnet work is to feed non-trivial target transitions into this producer, then exercise competing branches and synchronization against the same real state-transition code.
