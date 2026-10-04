# Memobi Mining

Mining is a native part of Memobi's intended node experience.

## First implementation

The first miner should be a Rust component, not a Flutter implementation.

Inputs:

- previous block hash;
- block height;
- transaction root;
- timestamp;
- PoARM version;
- epoch/algorithm parameters;
- current target.

The miner searches nonces and evaluates the selected PoARM candidate.

## Validation

A discovered block is only a candidate until the node validates:

- previous tip;
- height;
- timestamp rules;
- target;
- PoARM version;
- PoARM proof;
- transaction structure;
- transaction root;
- reward rules.

## Mobile

The Android client can later control mining intensity for battery and thermal safety. Those controls affect whether and how aggressively a device mines locally; they must never alter consensus validation.

## Pools

Solo mining is the initial direction. Pool protocols are not required for the first devnet.

## Research warning

PoARM v0 is experimental. Candidate A/B/C are laboratory variants and must not be treated as a finalized mainnet mining algorithm.
