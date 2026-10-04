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
- [ ] Genesis block
- [x] Provisional coinbase/reward validation boundary
- [x] Transaction validation skeleton
- [x] UTXO set skeleton
- [ ] Block validation
- [x] Add provisional cumulative-work scoring
- [x] PoARM header-seed construction
- [ ] PoARM mining
- [x] Difficulty-adjustment laboratory model
- [ ] Difficulty adjustment
- [x] P2P message serialization skeleton
- [x] Add bounded P2P collection/payload decoding
- [ ] P2P networking
- [x] Add provisional synchronization state and locator
- [x] Wallet key-selection requirements
- [ ] Wallet and key handling
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

- [ ] Public bootstrap infrastructure
- [ ] Testnet genesis
- [ ] Explorer
- [ ] Test MEMO distribution
- [ ] Independent node operation
- [ ] Network stress testing
- [ ] Adversarial testing

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
