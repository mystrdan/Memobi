# PoARM Candidate Families

## Purpose

The current PoARM implementation is a baseline laboratory workload. These candidate families define what we should test next; none is consensus-ready.

## Candidate A — Current baseline

- SHA-256 initialization
- memory allocation
- sequential memory updates
- pseudo-random indexed reads
- configurable rounds
- final SHA-256

Use this as the reference point for experiments.

## Candidate B — Dependency chain

Increase the amount of work that depends on the immediately preceding state.

Research questions:

- Does this reduce useful massive parallelism?
- How does ARM64 compare with x86-64?
- Does memory pressure remain practical on phones?
- Can verification remain cheap?

## Candidate C — Epoch parameterization

Derive workload parameters from a deterministic epoch seed.

Potential parameters include:

- memory size within a bounded range
- dependency pattern
- round count
- mixing schedule

The parameters must be derivable by every node from chain data. They cannot depend on miner-reported device properties.

## Candidate D — Sustained mobile profile

Tune a candidate specifically around sustained smartphone execution rather than short benchmark peaks.

Measure:

- work/second over long runs
- thermal throttling
- energy per unit of work
- single-core efficiency
- multi-core scaling

## Attack checklist

Every candidate should be tested against:

- GPU parallelization
- FPGA specialization
- ASIC specialization
- high-memory desktop CPUs
- server CPUs
- multi-device farms
- parameter precomputation
- memory-time tradeoffs

A candidate is not considered resistant merely because it performs well on one phone.

## Selection rule

Do not select a PoARM candidate from benchmark speed alone.

Selection requires a combination of:

1. mobile performance
2. sustained behavior
3. scaling analysis
4. verification cost
5. implementation simplicity
6. attack analysis
7. reproducibility across architectures
