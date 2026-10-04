# Android Preparation

## Status

Preparation only. The Android application UI is not being built yet.

## Core target

The Rust core should be portable to Android ARM64 so the same consensus implementation can be exercised on real phones.

Primary target:

- aarch64-linux-android

## Architecture direction

Keep protocol and consensus logic in Rust.

The future Android app should call the Rust core through a narrow, versioned interface rather than duplicating consensus rules in UI code.

## First Android milestone

Before building the all-in-one app UI:

1. compile memobi-core for Android ARM64
2. compile the PoARM laboratory for Android where practical
3. execute deterministic protocol tests on a physical ARM64 device
4. measure PoARM on sustained mobile hardware
5. only then choose the UI/application bridge

No Android-specific behavior may change block validation or other consensus rules.
