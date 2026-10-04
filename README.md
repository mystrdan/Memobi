# Memobi

**Memobi** is an experimental blockchain built around **PoARM (Proof of ARM)** — a Proof-of-Work design intended to make smartphone-class CPUs first-class mining hardware.

> A Proof-of-Work blockchain designed for the computers people already carry.

## Project status

Memobi is currently in the protocol and research phase.

We are deliberately starting with the consensus mechanism before building the wallet, node, explorer, or Android application.

### Initial goals

- Real Proof-of-Work, not simulated mining
- Smartphone/tablet-class CPUs as the intended mining hardware
- Permissionless participation
- Native MEMO currency
- Simple UTXO-based payments
- No smart contracts
- No EVM
- No NFTs or DeFi
- P2P-first network architecture
- Mobile-first node and mining experience

## Development language

Memobi's core implementation will be written in **Rust**.

Rust is selected for its low-level control, memory safety, concurrency guarantees, and suitability for high-performance systems software.

## Development phases

1. **Protocol specification**
2. **PoARM laboratory and benchmarks**
3. **Memobi devnet**
4. **Android alpha**
5. **Public testnet**
6. **PoARM hardening**
7. **Mainnet candidate**
8. **Mainnet**

## Repository

The repository is intentionally starting small. Consensus assumptions will be documented and tested before they are treated as production rules.

See:

- `docs/protocol.md`
- `docs/poarm.md`
- `docs/economics.md`
- `docs/roadmap.md`