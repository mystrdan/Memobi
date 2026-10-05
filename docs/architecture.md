# Memobi Architecture

## Current direction

Memobi is split into independent layers.

### Protocol core

Rust code responsible for:

- consensus data structures
- deterministic serialization
- transaction rules
- block validation
- UTXO state
- cryptography
- PoARM
- chain selection
- difficulty
- protocol networking

### Node

A standalone process that can:

- maintain chain state
- connect to peers
- validate blocks and transactions
- synchronize
- relay transactions
- mine

### Flutter client

The Android-first all-in-one application.

It presents wallet, activity, network, and mining controls while delegating protocol work to Rust.

### Explorer

A separate web application for public chain inspection.

It is not a dependency of the wallet or node.

## Trust boundaries

The UI must not be trusted for:

- transaction validity
- balances
- proof-of-work validity
- chain state
- signatures

The Rust core independently computes and verifies these.

## Centralization rule

A hosted service may improve discovery or user experience, but Memobi ownership and consensus must remain functional without a mandatory centralized backend.
### Persistence boundary

The current persistence layer keeps canonical block headers in an append-only, checksummed store. ChainState::apply_validated_block_with_store() stages consensus state first and only commits the in-memory state after the durable header append succeeds. Full block/UTXO persistence remains a separate step.

### Peer boundary

p2p::PeerSession now owns the protocol handshake/liveness phase independently of transport. It tracks version establishment, remote height, and Ping/Pong handling; socket transport and peer selection remain outside the core protocol state machine.
