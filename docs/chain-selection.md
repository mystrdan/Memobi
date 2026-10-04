# Chain Selection

Memobi currently uses a laboratory chain-work model so devnet synchronization can be developed before the mainnet rule is frozen.

## Current provisional model

Each block contributes a monotonic work score derived from its u64 target:

`work = floor((2^64 - 1) / (target + 1))`

Cumulative chain work is the sum of block work. A candidate chain with greater cumulative work is preferred over a lower-work chain.

This is **not mainnet consensus**. The final rule must freeze:

- target representation
- minimum and maximum target
- exact proof-to-work calculation
- genesis treatment
- invalid-block handling
- equal-work tie breaking
- reorganization limits
- interaction with difficulty adjustment

The implementation is intentionally isolated so those decisions can change without rewriting P2P or wallet code.
