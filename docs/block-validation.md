# Block Validation

Memobi now has a small header-validation boundary that can be exercised before the full node exists.

## Current checks

- non-empty block
- non-zero target
- matching transaction root
- genesis height/previous-hash rules
- later blocks reference the current tip and increment height
- supplied PoARM proof satisfies the provisional target comparison

Transaction-state validation remains in `ChainState::apply_block`, including UTXO spending and same-block transaction dependencies.

## Still to freeze

- timestamp/median-time rules
- exact PoARM seed construction
- canonical proof encoding
- target encoding and bounds
- coinbase/reward transaction rules
- signature authorization
- final transaction-root construction
- contextual validation across forks

This is a devnet/laboratory boundary, not a mainnet consensus specification.
