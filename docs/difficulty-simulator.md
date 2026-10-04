# Difficulty Simulator

## Status

Laboratory model only. This document does **not** define Memobi consensus.

## Purpose

Before freezing a difficulty formula, Memobi needs to test how candidate rules behave when:

- mobile miner participation grows or falls quickly;
- individual devices have widely different performance;
- timestamps contain bounded noise;
- large amounts of hash power arrive or leave between adjustment windows.

## Baseline model

For a window containing `N` blocks:

`observed_time = last_timestamp - first_timestamp`

`observed_interval = observed_time / N`

A simple proportional candidate is:

`new_target = old_target * observed_interval / target_interval`

The candidate must then be bounded to a configured minimum and maximum change per window.

This formula is intentionally incomplete. Timestamp rules, integer rounding, minimum/maximum targets, and exact window boundaries still need to be specified.

## Simulation scenarios

Every candidate should be tested against the same scenarios:

1. Stable miner population.
2. 2x miner participation.
3. 10x miner participation.
4. 10x miner participation followed by 90% miner departure.
5. Highly variable mobile CPU speeds.
6. Timestamp jitter within the permitted protocol range.
7. Sustained mobile throttling.
8. Very small initial miner population.
9. Very large miner population.
10. Adversarial timestamp behavior within protocol limits.

## Measurements

Record:

- target over time;
- actual block interval;
- blocks per adjustment window;
- overshoot/undershoot;
- time to recover after hash-rate changes;
- maximum target movement per adjustment;
- effect of timestamp noise;
- behavior at integer boundaries.

## Design constraint

Do not optimize only for a smooth graph. The final rule must be deterministic, independently implementable, resistant to timestamp manipulation, and stable across the expected range of Memobi mining participation.

## Next implementation

Build a pure Rust simulator with no networking, wallet, or Flutter dependencies. It should accept a deterministic sequence of synthetic hashrates/timestamps and emit candidate target changes. Candidate formulas can then be compared before one is considered for consensus.
