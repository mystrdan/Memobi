# Memobi

**Memobi** is an experimental **privacy-focused blockchain** built around **PoARM — Proof of ARM**.

Its core thesis is:

> Explore a real Proof-of-Work network designed around the computers people already carry, while treating privacy as a protocol-level design requirement.

## Current protocol direction

- Chain: **Memobi**
- Native coin: **MEMO**
- Consensus research: **PoARM**
- Core language: **Rust**
- Mobile UI: **Flutter**
- First app target: **Android**
- Transaction model: **UTXO**
- Network model: **peer-to-peer**
- Explorer: separate web application
- Privacy model: **privacy by design**
- Current block-time hypothesis: **10 seconds** on the current experimental parameter set

Memobi is deliberately not being designed as a general-purpose smart-contract chain.

There are currently:

- no smart contracts
- no EVM
- no NFTs
- no DeFi
- no token factory
- no mandatory cloud accounts
- no centralized backend requirement

The current focus is **MEMO, PoARM, UTXO transactions, validation, peer networking, synchronization, mobile participation, and privacy-preserving protocol foundations**.

## Privacy-chain direction

Privacy is a first-class architectural requirement.

The chain is being built so future privacy features can be added without redesigning the core protocol around plaintext application data. In particular:

- plaintext private messages should not be written directly to the blockchain
- private application payloads should be encrypted before transport/storage where applicable
- on-chain data should be minimized
- wallet/spending identity should not automatically become a universal application identity
- network and messaging metadata should be treated as privacy-sensitive
- future messaging and file-transfer features must preserve the privacy model

Messaging between Memobi addresses and MEMO-based charges for file attachments are **planned concepts, not current protocol features**.

## Current implementation status

The Rust core currently contains foundations for:

- deterministic binary encoding
- transaction and block structures
- UTXO validation
- Ed25519 transaction authorization
- wallet/key derivation foundations
- provisional Bech32-style addresses
- coinbase and reward validation
- timestamp and block-size limits
- provisional difficulty adjustment
- PoARM seed construction and experimental mining
- cumulative-work chain selection
- atomic fork replay
- bounded P2P message encoding/decoding
- peer handshake/session state
- bounded synchronization planning
- consensus header-only validation before block download
- translation from sync-planner requests into P2P wire messages
- bounded source-side header and block serving
- exponential-backoff header locators with genesis fallback
- canonical header persistence
- canonical block persistence including PoARM proofs
- consensus replay-based restart recovery
- header/block persistence consistency checks during restart recovery
- an in-process node engine
- real two-node synchronization integration coverage

The node engine deliberately does **not** own sockets. Transport can be added around the protocol engine without moving consensus logic into the networking layer.

## Persistence and recovery

Canonical blocks and PoARM proofs can be stored in an append-only block log, while canonical headers have their own durable log.

On restart, Memobi reconstructs UTXO and chain state by replaying persisted canonical blocks through the normal consensus validation path. It does not rely on an unverified persisted UTXO snapshot.

Durable block/header commits are staged so an in-memory chain is not advanced before the corresponding persistent records exist; partial dual-store writes are rolled back where possible.

## Application architecture

The planned all-in-one mobile application combines wallet, MEMO activity, network status, and mining controls in one Flutter interface.

Flutter is the UI layer only. Protocol, consensus, cryptography, wallet primitives, UTXO state, PoARM, and networking foundations remain in Rust.

The protocol is being designed so independent wallets/nodes can exist later; the mobile application is an all-in-one client, not a requirement for the network itself.

## Repository structure

- `core/` — Rust protocol, consensus, wallet, UTXO, P2P, synchronization, node, and persistence foundations
- `poarm/` — PoARM research, miner, and deterministic devnet producer
- `docs/` — protocol and research documentation
- `mobile/` — reserved Flutter application layer
- `.github/workflows/` — automated checks (currently paused; manual dispatch only)

## Network promotion

Memobi is deliberately moving through a short validation ladder rather than spending
indefinitely in Devnet:

**Devnet → Testnet → Mainnet candidate → Mainnet**

Devnet is for deterministic engineering and consensus-path testing. Testnet is for
real multi-node behaviour, synchronization, persistence, PoARM benchmarking, wallet
testing, and adversarial testing. Once those gates are satisfied, development should
shift to a mainnet candidate instead of continuing to accumulate experimental features
on Testnet.

Testnet now has a distinct network identity and genesis construction path; it is not
simply a Devnet label.

## Roadmap

See [docs/roadmap.md](docs/roadmap.md).

The immediate engineering priorities are:

1. harden the Rust protocol and persistence paths (without running CI until re-enabled)
2. complete header/block synchronization semantics and persistence consistency checks
3. implement real P2P transport
4. benchmark PoARM on ARM64 and x86-64
5. finalize wallet/address and consensus specifications
6. integrate the stable Rust core with Android/Flutter
7. design privacy-preserving messaging/file transfer without putting plaintext application data on-chain

## Status

Memobi is experimental.

**PoARM, economic parameters, serialization formats, cryptography/address choices, difficulty rules, genesis configuration, and P2P wire semantics remain subject to research and review. They are not mainnet-final.**

See:

- [Architecture](docs/architecture.md)
- [Protocol](docs/protocol.md)
- [PoARM](docs/poarm.md)
- [Synchronization](docs/sync.md)
- [Roadmap](docs/roadmap.md)