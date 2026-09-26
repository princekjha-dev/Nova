# Contributing to Nova

Thank you for contributing to Nova!

## Code Guidelines

1. **Rust Monorepo**: All core modules, crypto, discovery, transport, sync, and plugin crates must compile with zero errors and pass all tests (`cargo test --workspace`).
2. **Security First**: Never bypass signature verification or session authorization. Always zeroize private key seeds upon drop.
3. **No GPL Contamination**: Nova core is Apache 2.0. Do not embed copyleft GPL/AGPL libraries directly into core binary distributions. Use dynamic linking or clean protocol boundaries.
