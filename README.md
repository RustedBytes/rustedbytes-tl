# rustedbytes-tl

[![Crates.io](https://img.shields.io/crates/v/rustedbytes-tl.svg)](https://crates.io/crates/rustedbytes-tl)

A lightweight HTML parser written in Rust. It borrows input bytes and offers
allocation-free bounded parsing by default, or heap-backed DOMs and CSS queries
with `std`. The Rust crate is imported as **`tl`**.

This README describes the current Git source. The source contains breaking changes since the published `0.2.0` API;
the first release containing them must be `0.3.x` or later.
See `Cargo.toml` for the version selected for the next release.
The published [0.2.0 API reference](https://docs.rs/rustedbytes-tl/0.2.0/tl/)
does not describe all APIs below. Generate current API docs with
`cargo doc --features entities --open`.

## Getting started

Use the current source with heap-backed parsing:

```toml
[dependencies]
rustedbytes-tl = { git = "https://github.com/RustedBytes/rustedbytes-tl", features = ["std"] }
```

Commit your application's Cargo.lock to retain the resolved Git revision.

```rust
# #[cfg(feature = "std")] {
let html = "<ul><li><a href='/item'>Item</a></li></ul>";
let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
let handle = dom.query_selector("ul > li a[href]").unwrap().next().unwrap();
let node = handle.get(dom.parser()).unwrap();
assert_eq!(node.inner_text(dom.parser()), "Item");
let tag = node.as_tag().unwrap();
let href = tag.attributes().get("href").unwrap().unwrap();
assert_eq!(href.as_bytes(), b"/item");
# }
```

Keep `html` alive for every use of the borrowed DOM. If the DOM must own its
input, use the safe `tl::VDomGuard::parse(String, ParserOptions)` constructor.
See the [usage guide](https://github.com/RustedBytes/rustedbytes-tl/blob/master/docs/usage.md) for owned input, attributes, decoded text,
query errors and bounded parsing.

## Features

| Feature | Behavior |
| --- | --- |
| Default (none) | `no_std`, no allocation; explicit capacities in `parse`; bounded simple selectors |
| `std` | Heap-backed storage, owned DOMs, text extraction, structural CSS selectors |
| `entities` | Implies `std`; adds character-reference decoding through `decoded_inner_text` |
| `portable-simd` | Requires nightly Rust; optional SIMD parsing path, combinable with the above |

`__INTERNALS_DO_NOT_USE` exposes instrumentation internals and is not a supported
application feature. Stable builds should select features explicitly rather
than use `--all-features`, which enables nightly-only `portable-simd`.

## Scope and documentation

The parser does not implement the complete HTML5 tree-building algorithm.
Malformed HTML can produce a different tree from a browser; omitted `tbody`
elements are not inserted. CSS support is a subset, and text extraction does
not implement browser layout or visibility rules. HTML parsing and text
extraction do not sanitize untrusted content.

- [Usage guide](https://github.com/RustedBytes/rustedbytes-tl/blob/master/docs/usage.md): executable examples and API behavior.
- [Development guide](https://github.com/RustedBytes/rustedbytes-tl/blob/master/docs/development.md): feature checks, docs, benchmarks and troubleshooting.

## Provenance and license

Forked from [astral-tl](https://github.com/y21/astral-tl) and extended for bounded
`no_std` parsing and RustedBytes consumers. Licensed under MIT; see [LICENSE](https://github.com/RustedBytes/rustedbytes-tl/blob/master/LICENSE).
