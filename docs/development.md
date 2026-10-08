# Development and verification

Run commands from the repository root. `std` changes public signatures and
storage behavior, so checking only one feature configuration is insufficient.
CI pins Rust 1.99.0 for stable checks and checks compilation on 1.92.0.
The manifest does not declare an MSRV; the compatibility job is not an MSRV promise.

## Stable checks

```sh
cargo fmt --check
cargo test --no-default-features
cargo test --features std
cargo test --features entities
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets --features entities -- -D warnings
```

README examples are included in crate rustdoc. The usage guide is also included in crate rustdoc; Cargo runs its doctests
with the selected features.
To check documentation alone:

```sh
cargo test --doc --no-default-features
cargo test --doc --features std
cargo test --doc --features entities
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features entities
```

Examples hide feature guards in rendered documentation so the same source can
be tested for all three configurations. Keep each example's feature requirement
clear in its surrounding text. Verify local Markdown links when moving pages.

## Nightly and safety checks

Stable `--all-features` fails because `portable-simd` requires nightly. The CI
nightly is pinned separately:

```sh
cargo +nightly-2026-09-25 test --no-default-features --features portable-simd
cargo +nightly-2026-09-25 test --features entities,portable-simd
```

Install Miri for that nightly before focused safety checks:

```sh
rustup component add miri --toolchain nightly-2026-09-25
cargo +nightly-2026-09-25 miri test --lib --features std tests::audit
cargo +nightly-2026-09-25 miri test --lib --features std owned
cargo +nightly-2026-09-25 miri test --lib --features std inline
```

The workflow's manual `miri` input enables these focused groups. They exercise
known unsafe boundaries, not every possible program execution.

## Benchmarks

```sh
cargo bench --bench tl -- --test
cargo bench --features std --bench tl -- --test
cargo bench --features entities --bench audit -- --test
cargo bench --features entities --bench audit
cargo run --release --features entities --example audit_allocations
```

`--test` is a smoke check, not a timing measurement. `tl` benchmarks parsing;
`audit` benchmarks queries and text on a prebuilt DOM. The allocation example
measures requested allocation activity separately. See the
[audit report](rust-audit.md) for fixtures, baselines, confidence intervals and
limits. Reproduce baseline and candidate with identical toolchains, dependency
resolution, features and flags; avoid concurrent builds during timing runs.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Examples fail to find `rustedbytes_tl` | Import `tl`; the package and library names differ |
| `parse` requires const arguments | Enable `std`, or provide all six bounded capacities |
| `decoded_inner_text` is missing | Enable `entities`, which also enables `std` |
| Query is rejected | Check the supported CSS subset; rejection differs from an empty iterator |
| Tree differs from a browser | Review HTML5 recovery limits in the compatibility notes |
| Capacity error | Increase the relevant bounded budget or use heap-backed `std` parsing |

Docs-only paths trigger CI too. This workflow checks code and docs; it does not
publish a crate, bump the version or create a release.
