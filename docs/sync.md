# Chain Synchronization

Memobi's synchronization work is being built independently of the eventual network transport.

## Current layers

1. **Header locator** — identifies a local tip that can seed a header request.
2. **Header sync state** — discovers whether a peer reports a higher chain.
3. **Block sync state** — follows header discovery with block retrieval.
4. **Synced state** — reached when local height catches the best known height.

The current Rust state machine is deliberately small. It does not trust peer height as proof of a valid chain; received headers and blocks must still pass local validation and eventual chain-selection rules.

## Future requirements

- fork-aware locators
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
