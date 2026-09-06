# Independent Rust browser/server certification (DEN-3432)

Source PR: https://github.com/ores-otel/ores.otel.log/pull/56.
All manifests use an immutable source commit, not a branch or copied SDK.
Existing canonical/legacy conformance checks are preserved unchanged; these are
additive tests for the new adapter, not replacements for historical parity.

Native external tests exercise interleaved request isolation, child context,
privacy, duplicate/malformed headers, and exporter-failure response preservation.
Real Chrome tests exercise local non-Send sinks, cross-island context, clocks,
IDs, and same-origin redirect-safe Request creation. Framework fixtures compile
actual Leptos 0.8.20 CSR/hydration/SSR and Dioxus 0.7.10 web/SSR code. Framework
checks are compilation probes, not claims of deployed UI or end-to-end OTLP
collector delivery. The provider, exporter, queue and authentication remain
application-owned. No real customer data or credentials are used.

Run `cargo test --manifest-path web-bridge/Cargo.toml --features native` and
`wasm-pack test --headless --chrome web-bridge --features browser`.
