# Memobi Protocol Test Vectors

These vectors exercise deterministic encoding. They are not yet a complete consensus test suite.

## Transaction vector 0001

Input:

- version: 1
- one input
- previous txid: 07 repeated 32 times
- previous output index: 3
- unlocking data: ASCII sig
- one output
- value: 123
- spending condition: ASCII condition
- fee: 2

Expected canonical encoding, shown as hexadecimal:

01000000 01000000
0707070707070707070707070707070707070707070707070707070707070707
03000000 03000000 736967
01000000 7b00000000000000 09000000 636f6e646974696f6e
0200000000000000

The vector exists to catch accidental changes in field order, integer endianness, or length encoding.

## Block header vector 0001

A header with:

- version 1
- previous block hash: 01 repeated 32 times
- height 42
- timestamp 100
- target u64::MAX
- PoARM version 0
- PoARM nonce 9
- transaction root: 02 repeated 32 times

must encode to exactly 104 bytes.

Future vectors should include full block IDs, transaction IDs, signatures, addresses, PoARM proofs, difficulty transitions, and genesis data.
