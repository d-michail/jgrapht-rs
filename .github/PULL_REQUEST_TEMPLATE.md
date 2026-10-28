## What and why

<!-- What does this change, and why is it needed? Link the issue if there is one. -->

## Checklist

- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and
      `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` are clean
- [ ] Tests added or updated; the relevant suites pass
- [ ] Public items have doc comments (`# Errors` for fallible functions)
- [ ] Commits are signed off (`git commit -s`)
