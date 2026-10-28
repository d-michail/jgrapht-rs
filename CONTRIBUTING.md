# Contributing

Thank you for your interest in jgrapht-rs. Bug reports, fixes, new algorithms and documentation
improvements are welcome.

## Before you start

- For anything larger than a fix, open an issue first so the design can be discussed.
- The code follows the conventions in [`AGENTS.md`](AGENTS.md): typed errors instead of panics,
  capability bounds instead of run-time "unsupported" errors, documented public items, and tests
  that mirror the upstream JGraphT test classes where an upstream class exists.
- Contributions are accepted under the project's licence, `EPL-2.0 OR LGPL-2.1-or-later`
  (inbound = outbound). Sign off your commits with `git commit -s`
  ([Developer Certificate of Origin](https://developercertificate.org/)). Do not submit code from
  sources whose licence is incompatible. See [`CONTRIBUTORS.md`](CONTRIBUTORS.md).

## Building and testing

The workspace requires Rust 1.88 or newer.

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo test --workspace            # the algorithm suite alone takes several minutes
cargo test -p jgrapht --features serde --test serde_roundtrip
```

Benchmarks never run automatically; see the Benchmarks section of the [README](README.md).

## Pull requests

- Keep a pull request focused on one change and explain why it is needed.
- Add or update tests, and doc comments for every public item you touch (both crates deny
  `missing_docs`). Public fallible functions have an `# Errors` section.
- Make sure the commands above pass; CI runs them on stable and on the minimum supported Rust
  version.
