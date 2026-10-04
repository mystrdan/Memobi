# Consensus Checklist

Before Memobi can claim a mainnet-ready consensus specification, all of the following must be deterministic and independently implementable.

## Block

- [ ] canonical header serialization
- [ ] block identifier
- [ ] transaction serialization
- [ ] transaction root
- [ ] genesis block
- [ ] timestamp rules
- [ ] chain-selection rule

## Proof of Work

- [ ] final PoARM candidate
- [ ] PoARM versioning
- [ ] epoch transition rules
- [ ] exact target representation
- [ ] proof-to-target comparison
- [ ] difficulty adjustment
- [ ] cross-platform test vectors

## Transactions

- [ ] final signature primitive
- [ ] address format
- [ ] signature authorization
- [ ] coinbase/reward transaction
- [ ] fee rules
- [ ] in-block transaction dependencies
- [ ] UTXO atomicity

## Network

- [ ] peer handshake
- [ ] message limits
- [ ] peer scoring
- [ ] block/transaction relay
- [ ] header sync
- [ ] block sync
- [ ] reorg handling
- [ ] chain-selection integration

## Economics

- [ ] block interval
- [ ] emission schedule
- [ ] maximum supply decision
- [ ] fee policy
- [ ] reward validation

Nothing here should be marked complete merely because code exists. A consensus item is complete only when its behavior is specified, tested, and covered by deterministic vectors.
