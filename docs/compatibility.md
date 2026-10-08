# Compatibility and limits

This reference describes the current source. For complete method signatures,
run `cargo doc --features entities --open`; for examples, see the
[usage guide](usage.md). The crate parses HTML already supplied by the caller;
it does not fetch URLs, execute JavaScript or implement XPath.

## Choosing a build

| Contract | Default / `no_std` | `std` (also enabled by `entities`) |
| --- | --- | --- |
| HTML storage | Inline, fixed capacities | Heap-backed storage |
| `parse` | Six const capacity arguments | No const arguments required |
| DOM ownership | Borrows `&str` | Borrows `&str`, or owns a `String` through `VDomGuard` |
| DOM `query_selector` failure | `Err(ParseError::SelectorCapacityExceeded)` | `None` |
| Text extraction | Inspect raw nodes or write markup | `inner_text`; `decoded_inner_text` with `entities` |
| Structural CSS matching | Unavailable | Supported subset below |

Borrowing input avoids copying the entire HTML string. It does not mean that
`std` parsing, query construction or text extraction avoids allocations.
Neither build is a full browser DOM or an HTML sanitizer.

## CSS subset with std

| Selector | Example | Meaning |
| --- | --- | --- |
| Type, universal, ID, class | `a`, `*`, `#main`, `.item` | Match an element |
| Attribute presence | `[href]` | Attribute exists, including a valueless attribute |
| Attribute values | `[href='/']`, `[class~='item']`, `[href^='/']`, `[href$='.pdf']`, `[href*='docs']` | Equality, whitespace token, prefix, suffix, substring |
| Compound and list | `a.item[href]`, `p, a` | Intersection and union |
| Descendant and child | `main a`, `main > a` | Ancestor or direct parent |
| Adjacent and general sibling | `h2 + p`, `h2 ~ p` | Preceding element sibling |
| Position | `:first-child`, `:nth-child(2n+1)` | One-based element position; ignores text/comments |
| Negation | `a:not(.hidden)` | Exclude a matching selector |
| Descendant presence | `article:has(a)` | Require a matching descendant |

Type matching is ASCII-insensitive. CSS escapes, namespaces, attribute case
flags and other pseudo-classes are unsupported. Relative arguments such as
`:has(> a)` are rejected. Invalid queries return `None`; a valid query without
matches returns an empty iterator.

The parser rejects recursive selector calls at depth 64 and compounds with
64 or more simple components. These are implementation bounds, not a promise
that every smaller expression is valid. Structural queries construct indexes
proportional to DOM size; `:has` can scan many candidates.

For bounded builds, prefer individual tag, ID, class or universal selectors.
Compounds such as `a.item`, lists and combinators are rejected. A selector error
can mean unsupported syntax, so increasing capacity alone may not fix it.

## Bounded parse errors

The six `parse` capacities appear in the order shown below. Budgets include
text and comment nodes, not only HTML elements.

| Error | Budget or cause | Caller action |
| --- | --- | --- |
| `NodeCapacityExceeded` | First: all parsed nodes | Increase node capacity |
| `StackCapacityExceeded` | Second: parser stack entries | Increase nesting capacity |
| `RootCapacityExceeded` | Third: root nodes | Increase root capacity |
| `IdCapacityExceeded` | Fourth: tracked ID entries | Increase ID capacity or disable tracking |
| `ClassCapacityExceeded` | Fifth: tracked class storage | Increase class/node budgets or disable tracking |
| `SelectorCapacityExceeded` | Sixth: selector storage, or rejected selector syntax | Review syntax and selector budget |
| `AttributeCapacityExceeded` | 8 ordinary attributes per tag | Reduce attributes or use `std` |
| `ChildCapacityExceeded` | 256 direct children per tag | Reduce children or use `std` |
| `InvalidLength` | Input length exceeds `u32::MAX` bytes | Reject or split the input before parsing |

ID and class attributes have separate storage from the 8 ordinary attributes.
The six const arguments do not change the per-tag attribute/child bounds.
Large inline budgets also increase object and stack size. Capacity errors do
not establish that the HTML is malformed, and successful parsing does not
establish HTML5 conformance.

## HTML and output behavior

The parser does not implement the complete HTML5 tree-building algorithm.
It does not insert omitted `tbody` elements or reproduce all browser recovery
rules for misnested tags. Test selectors against your actual source HTML.

Raw text extraction concatenates descendant text without layout separators,
visibility filtering or entity decoding. Entity decoding is opt-in and does
not rewrite attributes. Serialized markup can differ from the original bytes;
retain the input when exact source formatting is needed.
