# Difficulty Adjustment

## Status

Research draft. The target interval and adjustment window are not frozen.

## Objective

Difficulty must react to changing aggregate Proof-of-Work while avoiding large oscillations caused by short-term timestamp or hash-rate changes.

A starting hypothesis is a 10-second target block interval with an adjustment window on the order of hours, but this is only a laboratory parameter.

## Requirements

The eventual rule should:

- use only deterministic chain data
- resist timestamp manipulation
- cap per-adjustment changes
- avoid abrupt difficulty cliffs
- work when miner participation grows from a few devices to very large populations
- remain simple enough for independent implementations
- be covered by executable test vectors

## Research variables

Model at least:

- target block interval
- adjustment window
- permitted timestamp range
- bounded adjustment ratio
- median-time or equivalent timestamp rule
- expected hash-rate growth
- sudden miner departure
- highly variable mobile performance

No final formula should be selected until PoARM performance and expected miner populations are modeled together.
