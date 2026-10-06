# Memobi Roadmap

## Phase 0 — Protocol specification

- [x] Establish repository
- [x] Choose Rust for core implementation
- [x] Define PoARM research objective
- [x] Choose UTXO direction
- [x] Establish deterministic encoding skeleton
- [x] Establish protocol test-vector format
- [x] Draft P2P architecture
- [x] Draft cryptography/addressing requirements
- [x] Draft difficulty-adjustment requirements
- [x] Prepare Android ARM64 integration direction
- [ ] Freeze transaction serialization
- [ ] Freeze block serialization
- [x] Define provisional chain-selection laboratory rule
- [ ] Define difficulty adjustment
- [ ] Define emission schedule
- [ ] Define address encoding
- [ ] Define genesis rules

## Phase 1 — PoARM laboratory

- [x] Create Rust workspace
- [x] Implement benchmark harness
- [x] Implement candidate PoARM v0
- [x] Add deterministic unit tests
- [ ] Add published cross-platform test vectors
- [x] Implement experimental PoARM candidate variants A/B/C
- [x] Add canonical transaction/block-header hashing helpers
- [ ] Benchmark ARM64
- [ ] Benchmark x86-64
- [ ] Measure memory usage
- [ ] Measure sustained performance
- [ ] Test multi-core scaling
- [ ] Test GPU and ASIC resistance assumptions
- [ ] Iterate algorithm

## Phase 2 — Devnet

- [x] Begin blockchain data structures
- [x] Provisional deterministic genesis builder
- [x] Provisional mining block builder with coinbase
- [x] Add provisional devnet block builder
- [x] Provisional deterministic genesis
- [ ] Final genesis block
- [x] Provisional coinbase/reward validation boundary
- [x] Transaction validation skeleton
- [x] UTXO set skeleton
- [x] Provisional block validation
- [ ] Final consensus block validation
- [x] Add provisional cumulative-work scoring
- [x] PoARM header-seed construction
- [x] End-to-end PoARM devnet mining smoke test
- [x] Provisional PoARM mining\n- [ ] Production PoARM mining
- [x] Difficulty-adjustment laboratory model
- [x] Provisional difficulty enforcement
- [ ] Final difficulty adjustment
- [x] P2P message serialization skeleton
- [x] Add bounded P2P collection/payload decoding
- [ ] P2P networking
- [x] Two-node synchronization integration
- [x] Full canonical block persistence and restart replay recovery
- [x] Add provisional synchronization state and locator
- [x] Add bounded synchronization request planner
- [x] Add persistent canonical-header storage foundation
- [x] Add in-process node engine joining peer, sync, consensus, and header persistence
- [x] Add canonical block storage with persisted PoARM proofs
- [x] Add restart recovery by replaying persisted canonical blocks through consensus
- [x] Wallet key-selection requirements
- [x] Provisional wallet and key handling\n- [ ] Final wallet/key specification
- [ ] CLI node and miner

## Phase 3 — Android alpha

- [x] Define Rust/Android ARM64 preparation path
- [ ] Compile Rust core for Android
- [ ] Wallet
- [ ] Sync
- [ ] Mining controls
- [ ] Device and network discovery
- [ ] Send and receive
- [ ] Activity
- [ ] Battery and thermal safety controls

## Phase 4 — Public testnet

The project is now moving toward a short public Testnet phase. Testnet must be a real
network with its own chain identity/genesis; it is not a renamed Devnet.

- [ ] Public bootstrap infrastructure
- [x] Testnet genesis construction with distinct chain identity
- [ ] Explorer
- [ ] Test MEMO distribution
- [ ] Independent node operation
- [ ] Network stress testing
- [ ] Adversarial testing

## Promotion gate

The intended progression is **Devnet → short Testnet → Mainnet candidate → Mainnet**.

Devnet is no longer the default destination for new architecture work. Testnet is used
to expose real multi-node, synchronization, PoARM, persistence, wallet, and adversarial
issues. We should not keep adding speculative features to Testnet once its core network
behaviour is proven.

Before Mainnet candidate, the remaining blockers are consensus/PoARM finalization,
reproducible genesis, real P2P transport, independent node operation, security review,
and cross-platform validation.

## Phase 5 — Mainnet candidate

- [ ] Consensus freeze
- [ ] Security review
- [ ] PoARM finalization
- [ ] Genesis reproducibility
- [ ] Mainnet client
- [ ] Documentation
- [ ] Independent validation

## Phase 6 — Mainnet

Only after the protocol and PoARM assumptions have survived public testing.


## Short Testnet gate

Before promoting the protocol to a Mainnet candidate, the current Testnet gate is intentionally narrow: independent nodes must handshake over the transport boundary, synchronize headers then blocks, converge on the same tip/UTXO state, survive durable restart recovery, and reject malformed or divergent persisted data. PoARM must also receive CPU/ARM64 measurements before its production parameters are frozen. This is a gate, not a second feature-development cycle.
