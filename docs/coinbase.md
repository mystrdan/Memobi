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

The current implementation estimates transaction fees from the current UTXO set before full transaction application. It is a development bridge, not a final consensus rule. Final validation must calculate fees from fully validated transactions and define coinbase maturity/spendability rules if needed.

Genesis remains special and must eventually receive explicit genesis/coinbase semantics rather than inheriting ordinary block rules.
