# `rustedbytes-tl`

[![Crates.io Version](https://img.shields.io/crates/v/rustedbytes-tl)](https://crates.io/crates/rustedbytes-tl)

tl is a fast HTML parser written in pure Rust.

By default this crate builds without `std` or `alloc`. Enable `std` for the
allocating convenience API:

```toml
rustedbytes-tl = { version = "0.1", features = ["std"] }
```

For the nightly portable SIMD path, enable `portable-simd` and build with
nightly:

```sh
cargo +nightly build --features portable-simd
```

## Queries and decoded text

Enable `std` for structural CSS queries and safe owned DOMs:

```rust
# #[cfg(feature = "std")] {
let guard = tl::VDomGuard::parse(
    "<ul><li><a href='/item'>Item</a></li></ul>".into(),
    tl::ParserOptions::default(),
).unwrap();
assert_eq!(guard.get_ref().query_selector("ul > li a[href]").unwrap().count(), 1);
# }
```

Enable `entities` to use `decoded_inner_text` for HTML character references.
`inner_text` continues to return the original undecoded text.
See [compatibility notes](docs/silkworm-compatibility.md) for supported selectors,
remaining HTML5 differences, query limits and release compatibility.

## Provenance

This crate is a fork of [`astral-tl`](https://github.com/astral-sh/astral-tl), modified to
add no-std, zero-copy parsing and other improvements.

## License

MIT.
