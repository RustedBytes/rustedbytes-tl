# Compatibility work for silkworm

Baseline: `6b6e0a6ed475c9011e3449ada93388493b6dc99c`, rustedbytes-tl 0.2.0.
The motivation is [silkworm #17](https://github.com/RustedBytes/silkworm/issues/17)
and its parser evaluation: valid structural selectors returned empty results;
unsupported pseudo-classes were rejected; text retained character references;
and the owned DOM entry point required an unsafe call.

## Changes

With `std`, query iterators now evaluate descendant (`a b`), child (`a > b`),
adjacent sibling (`a + b`) and general sibling (`a ~ b`) selectors using the
actual direct-child graph. Text and comments do not count as element siblings.
Compound selectors bind before combinators; combinators bind before lists.
Supported pseudo-classes include `:first-child`, `:nth-child(an+b)`, `:not(...)`
and descendant-relative `:has(...)`. Type names match ASCII-insensitively.
Quoted attribute values can contain spaces, punctuation and Unicode.
`*` now selects elements, excluding text and comments. Empty substring/prefix/
suffix attribute values do not match. Invalid syntax returns `None`.

Queries using structural selectors build an O(nodes) parent/sibling/position
index. Simple selectors keep the existing path without this index. `:has()`
scans candidates with a borrowed scope boundary, without copying the index per
anchor. See [Rust audit notes](rust-audit.md) for measured timing and allocations. Parsing is bounded to fewer than 64
recursive levels and fewer than 64 simple selectors per compound; larger inputs
return `None` instead of risking recursive stack exhaustion.

`VDomGuard::parse(String, ParserOptions)` exposes the existing guarded owner
through a safe constructor. It keeps the boxed input alive; DOM/node borrows
are bound to the guard, including cloned nodes. The guard's internal DOM drops
before the backing input. Existing `parse_owned` remains available with its
original signature. No new unsafe operation was introduced. A compile-fail
doctest verifies that a cloned node cannot outlive its owner.

The optional `entities` feature implies `std` and adds `web_atoms` 0.2.6, the
html5ever project's character-reference table. `Node::decoded_inner_text` and
`HTMLTag::decoded_inner_text` decode ordinary text, including two-codepoint
named entities, optional semicolons, numeric references, replacement characters
and legacy Windows-1252 numeric references. Decoding happens independently per
text node, so markup boundaries cannot manufacture references. Script/style
**tags** preserve raw text; a raw node queried directly uses text-context
rules. Existing `inner_text`, attributes and original source bytes are unchanged.
No dependency is added to the allocation-free default configuration.

## Remaining migration requirements

This remains a lightweight parser, not a complete HTML5 tree builder. It does
not insert implied `tbody` elements, perform foster parenting or implement the
adoption agency algorithm. For example, `table > tbody > tr` still differs from
scraper when the input omits `tbody`. A migration must either implement and
validate a complete recovery mode or explicitly accept the differing tree.
Adding a synthetic tbody alone would not establish HTML5 equivalence.

CSS escapes, namespaces, relative `:has(> a)`, other pseudo-classes, and
attribute case-sensitivity flags remain unsupported. Attribute matching uses
raw attribute values; the decoded-text API does not change this. Unsupported
queries must be handled explicitly by callers; empty results cannot signal
whether a different backend should be tried.

Silkworm's public APIs accepting `scraper::Selector` still require a compatibility
adapter or an intentional API migration. These changes alone do not remove
that dependency. Before replacement, run differential tests over real HTML
and benchmark full parse/query/text extraction with the adapter and ownership
model, rather than raw parse time alone.

The new public `Selector` variants affect exhaustive matches in downstream
code. Publish this as a breaking pre-1.0 release (0.3.x), not as a 0.2.x patch.
This PR does not publish a crate or change the release version.

## Verification

```sh
cargo test --features std
cargo test --features entities
cargo check --no-default-features
cargo +1.92.0 check --features entities
cargo +nightly-2026-09-25 check --features portable-simd
cargo fmt --check
cargo clippy --lib --features entities -- -D warnings -A clippy::byte_char_slices
```

The final clippy allowance is for the pre-existing byte-character array in
`src/parser/base.rs`, unrelated to this change. Tests cover the observed
combinators, mixed/list precedence, sibling numbering across text/comments,
negative and positive nth-child formulae, has scoping, malformed/overlarge
queries, quoted attributes, owner movement across a thread, and decoded-text
edge cases. The initial investigation did not run Miri. The subsequent
[Rust audit](rust-audit.md) covers panic cleanup, containers and owned DOMs under Miri.
