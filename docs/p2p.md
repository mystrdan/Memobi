# Memobi P2P Design

## Status

Architecture draft. Message names and limits are not consensus-frozen.

## Principles

- Peer-to-peer operation without a mandatory central server.
- Nodes validate what they receive instead of trusting peers.
- Bootstrap nodes may assist discovery but are not authorities.
- Keep mobile bandwidth, memory, and battery use practical.
- Separate discovery, synchronization, and propagation concerns.
- Never make wallet ownership depend on an account service.

## Initial peer lifecycle

1. discover candidate peers
2. establish a transport connection
3. exchange protocol version/capabilities
4. verify the peer is speaking Memobi
5. exchange chain knowledge
6. synchronize missing headers/blocks
7. propagate valid transactions and newly mined blocks

## Candidate messages

The first protocol will likely need equivalents of:

- version
- verack
- getheaders
- headers
- getblocks
- blocks
- inv
- tx
- ping
- pong

Names are provisional.

## Mobile considerations

A mobile node should be able to:

- synchronize incrementally
- limit concurrent peers
- sleep networking when appropriate
- resume synchronization safely
- avoid downloading duplicate data
- validate blocks locally

Client-side power/network policies must not alter consensus.

## Abuse resistance

Before public testnet, define:

- message size limits
- connection limits
- request rate limits
- duplicate suppression
- peer scoring
- timeout behavior
- invalid-data penalties

These controls protect availability; they do not determine who may mine.
