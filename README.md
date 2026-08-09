# ores-otel-log-rust-test

Exact-head conformance harness for **rust**.

This repository tests both `ores-otel/ores.otel.log` and `ORESoftware/next-loggers.ts` using explicit commit SHAs.
The required native command is recorded in `conformance.json`: `cargo test --all-features && cargo clippy --all-targets --all-features -- -D warnings`.
