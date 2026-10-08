# Rust audit and measured improvements

Baseline: merged PR #1, commit `46a4f9f`. The crate is edition 2024 with no declared
MSRV; compatibility is checked on Rust 1.92 without raising a compiler floor.
Current stable was verified from the [official release notes](https://doc.rust-lang.org/stable/releases.html):
Rust 1.99.0, released 2026-10-01. No dependency versions or allocator were changed.

The review applied correctness testing, unsafe auditing, idiom/API review,
standard-library and performance optimization, Criterion measurement, Rust
modernization, CI and documentation guidance. The library has no async runtime,
FFI or PyO3 boundary, so those workflows are not applicable. This is a focused
source and test review, not proof that every input/platform is correct.

## Confirmed defects and fixes

| Area | Trigger and consequence | Fix and evidence |
| --- | --- | --- |
| Inline raw storage, high severity | Safe `inline_parts_mut()` allowed replacing a live slot with uninitialized storage; later get/drop would access invalid values. | The public method now requires `unsafe` and documents initialization and unique ownership obligations. Two compile-fail examples enforce the safe-call boundary. |
| Map spill, high severity | A key's `Hash` or `Eq` panicked after an entry was moved into a temporary heap map, while the source still counted that slot as initialized. Unwinding could drop moved/uninitialized storage. | Reduce the initialized prefix before moving each entry. A `Hash` panic test counts all three destructors and then accesses/drops the remaining map; it passes Miri. |
| Container cloning, medium severity | A later `Clone` panicked, leaking earlier clones held in an unguarded MaybeUninit array. | Build into an owner with an incrementally updated initialized prefix. Both baseline regressions fail (0 destructor calls versus 2 expected); candidate regressions and Miri pass. |
| Bytes ordering, medium severity | Equal content in borrowed and owned storage compared non-equal under derived pointer-based Ord; BTreeSet could retain duplicate equal values. | Implement lexicographic byte-content Ord/PartialOrd, consistent with Eq/Hash. The baseline regression fails; candidate and Miri pass. |
| Duplicate map keys, medium severity | Re-inserting an existing inline key grew storage or spilled rather than replacing it, unlike the heap-backed map. | Replace existing values before checking capacity. Works with/without std. The HTML parser retains the first ordinary attribute using a dedicated insertion path. |

The raw-storage signature change is intentional and requires the planned breaking
0.3.x release. Existing safe code should use `get_mut`, `as_mut_slice` (vector),
or map insertion instead. Code that genuinely manipulates raw storage must use
an unsafe block and establish the documented obligations; simply wrapping the
old code in unsafe does not establish them. No release or version bump occurs
in this change. Ordinary parser/selector public signatures remain unchanged.
After a panic during map spill, entries may have been consumed; remaining
storage stays valid and each live owner is dropped once. Transactional rollback
is not promised.

The std-only allocator used for probes is an example, not a production allocator.
New production unsafe reads have local initialized-prefix proofs. The empty
MaybeUninit array now uses safe inline-const initialization.

## Performance boundaries and results

Fixture: 200 identical `<li><a>plain text</a></li>` rows inside `<ul>` (5,209
input bytes). Setup builds/parses the fixture outside each measured operation.
`has_200` parses the query string, builds its context, counts all matches and drops
the iterator. `decoded_text_200` extracts the entire ul's decoded text and drops
the returned string. Both benchmarks verify the expected outputs before timing.

The matcher now passes a borrowed boundary to descendant matching inside `:has`
instead of cloning three index vectors per anchor. Text extraction shares one
output buffer across nested tags; a single unchanged text leaf remains borrowed.
The aggregate output still allocates and grows. No general zero-allocation or
crawler throughput claim follows from these results.

Measurements: x86_64 Linux, AMD EPYC 9V74 shared virtual host, rustc 1.99.0
`b940084d7`, LLVM 23.1.1, default system allocator. Criterion 0.3.6 was retained
from the existing 0.3 requirement. Bench profile: opt-level 3, fat LTO, one codegen
unit; RUSTFLAGS unset. The baseline and candidate lockfiles are identical
(SHA-256 `fefdea9c09e825ee74b9d274cde8d27270ce38e8adb534d405d93a3020ba7ead`).

| Operation | Baseline estimates (two runs) | Candidate estimates (two runs) | Alloc calls before → after | Realloc calls before → after | Requested bytes before → after |
| --- | --- | --- | --- | --- | --- |
| has_200 | 276.58 / 271.05 µs | 118.05 / 122.14 µs | 606 → 6 | 0 → 0 | 4,832,160 → 24,160 |
| decoded_text_200 | 11.465 / 11.498 µs | 5.9473 / 5.9770 µs | 401 → 1 | 8 → 8 | 9,110 → 5,110 |

Each timing run uses 100 samples, 3 s warmup, approximately 5 s measurement,
95% confidence intervals, and retains outliers. Matched comparisons report
has time changes of [-58.176%, -57.327%] and [-56.364%, -54.840%]; text changes
of [-49.256%, -47.051%] and [-49.082%, -46.363%]. These are operation-specific
microbenchmarks on a shared host; compilation activity overlapped part of the
first candidate run. Run order was base/base/candidate/candidate rather than a
fully alternated experiment. Treat the repeated large changes as exploratory
support for the allocation mechanism, not portable speed guarantees.

Allocation probes run separately, single-threaded, with fixture/DOM creation
excluded and output destruction included. Alloc and alloc_zeroed are counted
together; realloc is separate. Requested bytes sum allocation sizes plus new
reallocation sizes, not live/peak memory or bytes copied. Peak memory and tail
latency were not measured. The probe also measures an unchanged single leaf
separately; no DOM/query setup is included in that operation.

Existing parsing benchmarks were also repeated. Final Example Domain estimates
were 2.4609/2.4591 µs against 2.7641/2.4055 µs baseline runs; PyPI Simple estimates
were 6.7912/6.4526 µs against 6.9033/6.7192 µs. Example Domain comparisons changed
sign (-5.5% then +4.0%); PyPI gave no significant change then -2.7%. The small,
mixed raw-parse differences are inconclusive on this shared host. An earlier
prototype showed a PyPI regression; the final first-attribute insertion path
is inlined and did not reproduce that regression. These safety fixes do not
imply all parsing workloads become faster.

Reproduce with the same bench/example files and dependency lockfile in both
checkouts; never compare different fixtures. Baseline names must be unique:

```sh
cargo bench --features entities --bench audit -- --save-baseline base-SHA --noplot
# Rebuild candidate in the same target directory, then:
cargo bench --features entities --bench audit -- --baseline base-SHA --noplot
cargo run --release --features entities --example audit_allocations
cargo bench --features std --bench tl -- --test
```

Full measurements belong on controlled hardware. CI runs smoke checks only.
Generated target reports and logs are not committed.

## Validation and limits

Run stable configurations separately; all-features includes nightly-only SIMD:

```sh
cargo test --no-default-features
cargo test --features std
cargo test --features entities
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets --features entities -- -D warnings
cargo +1.92.0 check --all-targets --no-default-features
cargo +1.92.0 check --all-targets --features entities
cargo +nightly-2026-09-25 test --features entities,portable-simd
cargo +nightly-2026-09-25 miri test --lib --features std tests::audit
cargo +nightly-2026-09-25 miri test --lib --features std owned
cargo +nightly-2026-09-25 miri test --lib --features std inline
cargo fmt --check
```

The Miri groups passed (7, 7 and 22 test executions, with overlapping clone
regressions). Miri covers those executions on one toolchain/target; it is not a
soundness proof. The raw-mutation misuse is checked at compile time rather than
executing undefined behavior. The unsafe map spill reproducer was only executed
after the fix; its old failure follows from the ownership/initialized-prefix
trace. No claim of baseline Miri failure is made.

Strict standard Clippy checks pass. Pedantic was also inspected; numerous
warnings about existing underscore fields, missing convenience annotations and
fixture formatting remain, and pedantic is not made a blanket release gate.
No cargo-semver-checks or formal-verification result is claimed. Cross-platform
execution and full HTML5 recovery remain outside this review.

CI runs on relevant source/manifest/bench/example/README/workflow changes, PRs
and master pushes, with cancellation of superseded runs. It does not run full
performance measurements or publish releases. Miri is an optional manual input.
Do not mark a path-filtered workflow as an unconditional required check without
adding an always-reporting dispatcher: documentation-only changes may skip it.
The 1.92 job is compatibility evidence, not a declaration of an undeclared MSRV.
