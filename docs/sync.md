# Chain Synchronization

Memobi's synchronization work is being built independently of the eventual network transport.

## Current layers

1. **Header locator** — builds a bounded exponential-backoff locator from canonical headers, always retaining genesis.
2. **Header sync state** — discovers whether a peer reports a higher chain.
3. **Block sync state** — follows header discovery with block retrieval.
4. **Synced state** — reached when local height catches the best known height.

The current Rust state machine is deliberately small. It does not trust peer height as proof of a valid chain; received headers and blocks must still pass local validation and eventual chain-selection rules.


## Implemented synchronization planning

The core now exposes a bounded `SyncPlanner`. It separates peer height discovery from block application and produces either a header request or a bounded block batch. Peer-reported height only drives scheduling; received headers and blocks still require local consensus validation before the local height advances.

Block requests are capped by `SyncLimits::max_block_batch`, and a planner never requests beyond the remaining height gap. This is intended to be usable by both a direct network transport and a mobile resumable transport later.

## Persistent chain data

`storage::HeaderStore` provides an append-only, checksummed on-disk header log. `storage::BlockStore` now persists canonical blocks together with their PoARM proofs. Restart recovery replays those blocks through the normal consensus validation path to reconstruct UTXOs rather than trusting a persisted state snapshot.

## Future requirements

- fork-aware locators and common-ancestor selection
- durable block/state consistency checks
- multiple peers
- duplicate suppression
- bounded requests
- resumable mobile synchronization
- peer timeouts
- invalid-data penalties
- cumulative-work comparison
- reorganization handling
- persistent sync progress where useful

No centralized API is required by this design.


## Header-first validation

Incoming `Headers` payloads are decoded into canonical `BlockHeader` values and checked against the local chain before the planner advances to block synchronization. Header-only validation checks network-specific genesis identity, block and PoARM versions, target bounds, parent linkage, height continuity, timestamp ordering, and deterministic difficulty. It deliberately does not replace final block validation: the later block path still verifies the transaction body, transaction root, UTXO transitions, coinbase rules, and PoARM proof.

The sync planner's `GetBlocks { start_height, count }` request now maps directly to the P2P wire message. Request counts are bounded by the protocol's collection limit.


## Current source-side serving

The in-process node can now answer bounded `GetHeaders` requests from its canonical header history and bounded `GetBlocks` requests from canonical durable block storage. Header responses are capped at 2,000 headers and block responses at 256 blocks. The serving layer remains transport-neutral; sockets are still outside the node engine.
