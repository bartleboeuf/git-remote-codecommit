# Changelog

## v1.0.1
- Bump package version to `1.0.1` and raise minimum Rust toolchain to `1.93.0` while updating AWS/chrono/tokio dependencies and the bundled `build.sh` base image so the build stays aligned with the updated ecosystem.
- Harden CodeCommit signing by sorting/enumerating regions, adding the `eusc-de-east-1` sovereign endpoint, and refactoring signature generation to reuse fixed-size buffers and binary search for region lookup; this eliminates heap churn, trims string allocations, and keeps the canonical request consistent.
- Simplify profile parsing and URL handling so the helper reads credentials straight from the user-info payload without extra allocations or fallback heuristics.

## v1.0.0
- Initial release.