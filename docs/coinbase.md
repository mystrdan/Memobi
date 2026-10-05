# Coinbase and Mining Rewards

Memobi now has a provisional block-reward boundary for devnet work.

## Current model

The first transaction in a block is treated as the coinbase transaction:

- it has no inputs
- it has at least one output
- its fee field is zero
- it is the only coinbase transaction in the block
- its total output value may not exceed the provisional block subsidy plus transaction fees

The current laboratory subsidy uses the existing provisional halving model in `core/src/reward.rs`.

## Important limitation

The validator now evaluates ordinary transactions against a staged UTXO view so fees from earlier transactions in the same block can be accounted for. This remains a development bridge until transaction authorization, coinbase maturity/spendability, and final reward rules are frozen.

Genesis remains special and must eventually receive explicit genesis/coinbase semantics rather than inheriting ordinary block rules.


## Height-unique coinbase marker

Normal block coinbases now contain one reserved marker input rather than an empty
input list. The marker uses a zero transaction ID, the reserved index
`u32::MAX`, and the little-endian block height as unlocking data.

This commits the block height into the coinbase transaction ID. Two blocks paying
the same key therefore cannot accidentally recreate the same coinbase outpoint.

Genesis remains a special zero-input transaction and is handled separately by
the genesis validation path.

The marker is provisional consensus and should remain covered by deterministic
test vectors before any mainnet freeze.
