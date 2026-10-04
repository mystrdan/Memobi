# Memobi Node CLI

The first node CLI is a development tool for exercising the Rust core.

## Initial commands

Planned commands:

- `memobi init` — initialize local node data;
- `memobi status` — show chain/network state;
- `memobi mine` — run the local PoARM miner;
- `memobi peers` — inspect connected peers;
- `memobi blocks` — inspect local chain height/tip;
- `memobi wallet` — inspect the local wallet interface once wallet crypto is finalized.

## Design

The CLI must use the same Rust core as the Android application.

It must not contain a second implementation of:

- transaction validation;
- UTXO rules;
- block validation;
- PoARM;
- difficulty;
- wallet signing.

The CLI is an operator surface over the protocol core, not a separate protocol implementation.

## Development order

1. in-memory devnet;
2. deterministic genesis;
3. local mining;
4. block validation;
5. persistence;
6. P2P networking;
7. sync;
8. wallet commands.

No production daemon behavior is implied yet.
