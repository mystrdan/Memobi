# Memobi

**Memobi** is an experimental blockchain built around **PoARM — Proof of ARM**.

Its goal is simple:

> Explore a real Proof-of-Work network designed around the computers people already carry.

## Current direction

- Chain: **Memobi**
- Native coin: **MEMO**
- Consensus research: **PoARM**
- Core language: **Rust**
- Mobile UI: **Flutter**
- First app target: **Android**
- Transaction model: **UTXO**
- Network: **peer-to-peer**
- Explorer: separate web application

Memobi is deliberately not being designed as another general-purpose smart-contract chain.

There are currently:

- no smart contracts
- no EVM
- no NFTs
- no DeFi
- no token factory
- no mandatory accounts or cloud wallet

The focus is **MEMO, Proof-of-Work, payments, validation, networking, and mobile participation**.

## Application architecture

The planned all-in-one mobile application combines wallet, MEMO activity, network status, and mining controls in one Flutter interface.

Flutter is the UI layer. The protocol and consensus implementation remain in Rust.

## Status

Memobi is experimental. The PoARM algorithm, economic parameters, serialization, cryptography, difficulty rules, and genesis configuration are still being researched and must not be treated as mainnet-final.

## Repository structure

- `core/` — Rust protocol primitives
- `poarm/` — PoARM research and benchmark code
- `docs/` — protocol and research documents
- `mobile/` — reserved Flutter application layer
- `.github/workflows/` — automated Rust checks

## Roadmap

See [docs/roadmap.md](docs/roadmap.md).

The immediate priority is to validate the protocol foundations and PoARM on real hardware before building the production mobile UI.
