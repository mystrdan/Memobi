# Memobi Mobile

This directory reserves the application layer for the future Flutter client.

## Current status

UI implementation has not started.

The intended stack is:

- Flutter for UI
- Rust for Memobi protocol/core
- Android as the first target

The Rust core must remain usable without Flutter so command-line nodes, test tools, and other clients can exist independently.

## Initial application surfaces

1. Wallet
2. Home / MEMO balance
3. Send
4. Receive
5. Activity
6. Mining
7. Network
8. Settings

These are product surfaces, not consensus rules.
