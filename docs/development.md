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

Docs-only paths trigger CI too. `ci.yml` checks code and docs. Version changes and publishing use the separate
workflows described below.


## Version changes and crates.io publishing

1. Run **Update version** (`update-version.yml`) from `master`. Choose `patch`,
   `minor` or `major`, or enter an explicit `MAJOR.MINOR.PATCH`. The workflow
   rejects malformed, unchanged or decreased versions and existing version
   branches/tags. For the breaking changes since 0.2.0, choose `minor` (0.3.0)
   or a later compatible release version, rather than a 0.2.x patch.
2. Review the generated `chore/version-*` PR and its explicitly dispatched
   `ci.yml` run, then merge it. Only `Cargo.toml` is committed: this library
   does not track `Cargo.lock`. Enable **Allow GitHub Actions to create and
   approve pull requests** in repository Actions settings. Branch protection
   may require additional checks on the PR itself; a manually dispatched run
   does not necessarily satisfy every required-check policy.
3. Create a tag named exactly `v<package version>` on the merged commit and
   publish a GitHub Release for it. Tag creation alone does not publish the
   crate. Publish the Release manually or with credentials that trigger
   Actions; Release events generated with `GITHUB_TOKEN` do not trigger this
   workflow.
4. **Publish crates.io** (`publish-crates.yml`) runs for a published stable
   Release. It checks the tag against `Cargo.toml`, requires its commit to be
   reachable from `master`, runs default/std/entities tests and verifies the
   packaged crate with default and entities features before uploading.
   Prereleases are skipped. Retrying cannot overwrite an existing crates.io
   version; Cargo reports an error if that version is already published.

Configure [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
for the existing `rustedbytes-tl` crate before the first automated publish:

| Setting | Value |
| --- | --- |
| GitHub owner | `RustedBytes` |
| Repository | `rustedbytes-tl` |
| Workflow filename | `publish-crates.yml` |
| GitHub environment | `crates-io` |

Create the matching `crates-io` environment in repository settings. The publish
job requests a short-lived token through `rust-lang/crates-io-auth-action`;
no long-lived `CARGO_REGISTRY_TOKEN` secret is required. The action revokes the
token when the job finishes. Restrict the environment to release tags if desired.
These workflows prepare future releases; adding them does not publish a version.
