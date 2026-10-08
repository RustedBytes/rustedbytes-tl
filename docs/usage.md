# Usage guide

These examples target the current Git source. Import the package
`rustedbytes-tl` as `tl`. Enable `std` for heap-backed parsing, and `entities`
for decoded text; the latter already enables `std`.

## Borrowed and owned input

`tl::parse(&str, ParserOptions)` borrows the input. Node handles must be resolved
against the parser that produced them; a handle is not an independently owned
node or a globally unique identifier. Do not use handles from another DOM.

For input obtained as a `String`, the guard keeps that input alive:

```rust
# #[cfg(feature = "std")] {
let guard = tl::VDomGuard::parse(
    String::from("<p id='message'>Hello</p>"),
    tl::ParserOptions::default(),
).unwrap();
let dom = guard.get_ref();
let handle = dom.get_element_by_id("message").unwrap();
assert_eq!(handle.get(dom.parser()).unwrap().inner_text(dom.parser()), "Hello");
# }
```

DOM references and cloned nodes obtained from the guard cannot outlive it.
`get_mut_ref` also returns a shared DOM reference; its name does not grant DOM
mutation. The legacy `unsafe parse_owned` entry point remains available, but
new code should use `VDomGuard::parse`.

`ParserOptions::default()` does not build ID/class indexes. With `std`,
`get_element_by_id` and `get_elements_by_class_name` still work by scanning.
Use `ParserOptions::default().track_ids().track_classes()` when repeated
lookups justify building indexes. CSS query support does not require these flags.

## Queries and attributes

With `std`, `query_selector` returns `None` for invalid or unsupported syntax.
A valid query returns an iterator, which can be empty:

```rust
# #[cfg(feature = "std")] {
let dom = tl::parse("<button disabled title='Save'>Save</button>", Default::default()).unwrap();
assert!(dom.query_selector("button:unsupported").is_none());
assert_eq!(dom.query_selector("article").unwrap().count(), 0);
let button = dom.query_selector("button[disabled]").unwrap().next().unwrap();
let attributes = button.get(dom.parser()).unwrap().as_tag().unwrap().attributes();
assert!(attributes.get("missing").is_none());
assert!(matches!(attributes.get("disabled"), Some(None)));
assert_eq!(attributes.get("title").unwrap().unwrap().as_bytes(), b"Save");
# }
```

The nested attribute result distinguishes missing attributes (`None`), present
attributes without a value (`Some(None)`) and values (`Some(Some(bytes))`).
Attributes retain raw character references. `Bytes::as_bytes()` gives bytes;
`try_as_utf8_str()` validates UTF-8 and `as_utf8_str()` (with `std`) converts
lossily. Initial spans borrow input; explicitly replacing bytes can own data.

Supported `std` queries include type, ID, class, universal, attribute presence
and value matching (`=`, `~=`, `^=`, `$=`, `*=`), compound selectors, comma lists,
descendant, child and sibling combinators, `:first-child`, `:nth-child(an+b)`,
`:not(...)`, and descendant-relative `:has(...)`. Element positions ignore
text and comments. Type names match ASCII-insensitively. CSS escapes,
namespaces, attribute case flags, other pseudo-classes and `:has(> a)` are
unsupported. Recursive nesting and compound size are bounded; see the
[compatibility notes](https://github.com/RustedBytes/rustedbytes-tl/blob/master/docs/compatibility.md).

Structural queries build indexes proportional to DOM node count. `:has` can
scan many candidates; it is not guaranteed to take linear time. Calling
`Selector::matches` directly lacks the structural context supplied by the DOM
query iterator.

## Walking the tree and writing markup

`dom.nodes()` includes every parsed node in source order, including text and
comments. `dom.children()` returns root handles; `tag.children().top()` returns
only the tag's direct child handles. Resolve each handle with `dom.parser()`.

```rust
# #[cfg(feature = "std")] {
let dom = tl::parse("<section><p>Hello</p><!--note--></section>", Default::default()).unwrap();
let root = dom.children()[0].get(dom.parser()).unwrap().as_tag().unwrap();
assert_eq!(root.children().top().len(), 2);
assert_eq!(dom.nodes().len(), 4);
let mut markup = String::new();
dom.write_outer_html(&mut markup).unwrap();
assert_eq!(markup, "<section><p>Hello</p><!--note--></section>");
# }
```

`write_outer_html` accepts a `core::fmt::Write` sink in either build. A `String`
sink allocates; use a bounded or streaming sink when that matters. Sink failures
return `core::fmt::Error` and can leave partial output. Serialization reconstructs
markup, so attribute quotes, ordering and other formatting can differ from the
input. It does not escape replacement attribute values or sanitize markup;
keep the original input if exact source preservation is required.

## Raw and decoded text

`inner_text` concatenates descendant text, ignores comments and leaves entities
encoded. It does not insert layout spaces or filter hidden elements. With
`entities`, opt into decoding:

```rust
# #[cfg(feature = "entities")] {
let dom = tl::parse("<p>A &amp; B &#x1F980;</p>", Default::default()).unwrap();
let handle = dom.query_selector("p").unwrap().next().unwrap();
let node = handle.get(dom.parser()).unwrap();
assert_eq!(node.inner_text(dom.parser()), "A &amp; B &#x1F980;");
assert_eq!(node.decoded_inner_text(dom.parser()), "A & B 🦀");
# }
```

Decoding is per text node: markup boundaries cannot form a new entity.
Script/style tags preserve raw text; directly decoding a raw child uses ordinary
text-context rules. Decoding does not change source bytes or attributes.
Returned text is `Cow<str>` and may borrow or allocate depending on the content
and tree. Do not assume every extraction is allocation-free.

## Bounded parsing without std

Without `std`, `parse` takes six capacities in this order: nodes, parser stack,
root nodes, tracked IDs, tracked classes and selector nodes. Include text and
comment nodes in the node budget. Capacities also affect stack/storage size;
choose them for your input rather than copying large arbitrary bounds.

```rust
# #[cfg(not(feature = "std"))] {
let dom = tl::parse::<32, 16, 8, 8, 8, 16>(
    "<p id='message'>Hello</p>", tl::ParserOptions::default(),
).unwrap();
assert!(dom.get_element_by_id("message").is_some());
assert_eq!(dom.query_selector("p").unwrap().count(), 1);
# }
```

Bounded selectors support simple matching, not the structural CSS engine
available with `std`. Here `query_selector` returns `Result`, rather than the
`std` API's `Option`. The bounded parser also reports invalid or unsupported selector syntax as
`SelectorCapacityExceeded`; that error does not prove that increasing capacity
will make the query valid.

Insufficient storage returns `ParseError`: node, stack, root, ID, class and
selector capacity errors identify those budgets. Without `std`, each tag can store 8 ordinary attributes (ID and class
are stored separately) and 256 direct children. These fixed bounds are not
changed by the six `parse` capacities. `InvalidLength` rejects input too large for the parser's `u32` spans.
These errors are not HTML conformance validation. For no-allocation output,
use `write_outer_html` with a caller-provided `core::fmt::Write` sink.

## Updating consumers

The current source adds public `Selector` variants, changes bounded parsing
APIs and makes `InlineVec::inline_parts_mut` and
`InlineHashMap::inline_parts_mut` unsafe. Exhaustive enum matches and callers of
raw mutable storage need review. Prefer safe collection methods such as
`get_mut`, `as_mut_slice` (vector) and `insert` (map). Do not add an unsafe block
without meeting the documented initialized-prefix and ownership invariants.
The manifest now declares `0.3.0`. Review the revision and features you consume;
`std` and bounded builds expose different parsing and query signatures.
