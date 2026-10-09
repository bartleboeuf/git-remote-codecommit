# Changelog

## v1.3.2
- fix plain `codecommit://` URLs forcing `us-east-1`; the region now comes from `AWS_REGION` or the profile, with `us-east-1` as fallback
- reject a profile region that is not a supported CodeCommit region, with a clear error
- detect expired SSO sessions from the full error chain and suggest `aws sso login --profile <name>`
- list available regions from the known-regions table, so the message can't drift
- rename the first argument to `remote_name` (Git passes the remote name, not the command)
- replace `chrono`, `hex`, `urlencoding`, and `aws-types` with small local helpers and re-exports from `aws-config`
- use an explicit single-threaded Tokio runtime instead of `#[tokio::main]`, reducing Tokio features to `rt`
- add unit tests for timestamp formatting, percent-encoding, and hex encoding

## v1.3.1
- update dependencies and raise minimum Rust toolchain to `1.96.0`
- update build workflow

## v1.3.0
- improve error handling and documentation

## v1.2.0
- add new regions + code optimizations
- update dependencies

## v1.1.0
- change workflow for windows build
- improve error messages and performances
- update known regions for codecommit endpoints

## v1.0.1
- Bump package version to `1.0.1` and raise minimum Rust toolchain to `1.93.0` while updating AWS/chrono/tokio dependencies and the bundled `build.sh` base image so the build stays aligned with the updated ecosystem.
- Harden CodeCommit signing by sorting/enumerating regions, adding the `eusc-de-east-1` sovereign endpoint, and refactoring signature generation to reuse fixed-size buffers and binary search for region lookup; this eliminates heap churn, trims string allocations, and keeps the canonical request consistent.
- Simplify profile parsing and URL handling so the helper reads credentials straight from the user-info payload without extra allocations or fallback heuristics.

## v1.0.0
- Initial release.