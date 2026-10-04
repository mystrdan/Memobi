# PoARM Benchmarking

The benchmark is intended to answer one question:

> Does the candidate workload create a useful economic and performance profile for smartphone-class CPUs?

## Rules

Do not compare raw work rates across different PoARM versions unless the version and configuration are recorded.

Every benchmark result should record:

- PoARM version
- commit SHA
- CPU model
- CPU architecture
- operating system
- core count
- memory
- PoARM memory allocation
- rounds
- samples
- elapsed time
- work/second
- sustained performance
- battery and thermal observations when available

## First benchmark matrix

### Mobile ARM64

- Snapdragon
- Dimensity
- Exynos
- Tensor
- Unisoc
- Apple Silicon

### Other hardware

- desktop x86-64
- laptop x86-64
- ARM desktop
- Raspberry Pi-class ARM
- ARM server

The objective is not to make other hardware unable to run PoARM. The objective is to investigate whether smartphone-class hardware can occupy the intended economic sweet spot.

## Sustained testing

A short benchmark is insufficient.

Mobile SoCs can change frequency and performance under sustained thermal load, so later experiments must include longer runs and thermal observations.
