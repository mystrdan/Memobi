# Memobi Flutter Application

## Decision

The Memobi user interface will be built with Flutter.

Flutter is the presentation layer only. Consensus, wallet cryptography, transaction construction, chain validation, synchronization, and PoARM remain in the Rust core.

## Target platforms

The first application target is Android.

The architecture should keep the UI layer portable so additional desktop or mobile targets can be considered later without changing consensus rules.

## Boundary

Preferred direction:

Flutter UI
→ narrow application API / FFI boundary
→ Rust Memobi core
→ networking, wallet, UTXO, validation, and mining

Flutter must not reimplement consensus logic.

## All-in-one application

The first Memobi app is intended to combine:

- wallet
- MEMO balance
- send / receive
- transaction activity
- node/network status
- mining controls
- PoARM status
- basic settings

There is no requirement for a cloud account, hosted wallet, or centralized backend.

## UI principles

- mobile-first
- simple
- fast
- clear status
- low battery impact outside active mining
- no unnecessary blockchain-dashboard complexity
- no smart-contract UI
- no DeFi/NFT surfaces

## FFI rule

The Rust/Flutter interface should expose stable application-level operations rather than raw consensus internals. Consensus data structures stay owned by Rust.

The exact bridge technology is intentionally deferred until the Rust core and Android build target are working.
