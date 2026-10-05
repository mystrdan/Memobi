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

## Repository replay path

Fork replay is performed against a scratch `ChainState` and is committed only when the candidate chain wins the current chain-selection rule. The parameterized `replay_fork_with_params()` path uses the supplied `ConsensusParams` for every block, while the shorter `replay_fork()` helper remains a devnet convenience wrapper.

This keeps reorganization validation atomic: an invalid fork cannot partially replace canonical UTXO/header state. The current equal-work-by-length behavior remains explicitly devnet/provisional and is not a mainnet consensus commitment.

## Reorganization state safety

There is deliberately no public partial-truncation helper for reorgs. A reorganization must be reconstructed through `replay_fork_with_params()`, which validates the complete candidate from genesis in scratch state and commits the entire resulting `ChainState` atomically. This prevents headers, tip, cumulative work, and UTXOs from becoming inconsistent during a rollback.
