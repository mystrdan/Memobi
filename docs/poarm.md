# PoARM — Proof of ARM

## Status

PoARM is experimental. This document defines research goals rather than a finalized mining algorithm.

## Objective

PoARM is intended to make smartphone and tablet-class CPUs the natural Proof-of-Work hardware for Memobi.

The goal is economic alignment, not an impossible claim that other machines can never execute the algorithm.

## Design requirements

A candidate PoARM design should investigate:

- CPU-oriented computation
- memory pressure appropriate for mobile devices
- sequential dependencies that reduce unlimited parallel scaling
- randomized or epoch-dependent parameters
- efficient verification
- deterministic results across architectures
- low implementation complexity
- resistance to straightforward ASIC specialization
- resistance to GPU-style massive parallelism
- practical execution on ARM64 mobile SoCs
- reasonable execution on desktop and server CPUs without making them the obvious dominant hardware

## Mobile-first benchmark

The first implementation will be a standalone benchmark rather than a blockchain miner.

Measure:

- hashes per second
- energy per unit of work where measurable
- memory consumption
- sustained performance
- thermal behavior
- performance degradation over time
- single-core and multi-core scaling
- ARM64 versus x86-64

## Hardware classes

The laboratory should eventually compare Snapdragon, MediaTek Dimensity, Samsung Exynos, Google Tensor, Unisoc, Apple Silicon, Raspberry Pi-class ARM systems, desktop CPUs, server CPUs, GPUs, and experimental ASIC implementations if relevant.

## Security principle

PoARM must remain ordinary Proof-of-Work at the consensus layer. The network must not require identity, Proof-of-Personhood, centralized hardware attestation, or permission from a manufacturer to mine.

## Research loop

1. implement candidate
2. benchmark
3. analyze scaling
4. attack the design
5. revise
6. benchmark again
7. publish assumptions
8. only then integrate into consensus
