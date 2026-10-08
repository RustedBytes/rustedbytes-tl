use super::*;
use crate::inline::{hashmap::InlineHashMap, vec::InlineVec};
use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

#[test]
fn bytes_order_matches_equality_and_contents() {
    let source = String::from("same");
    let borrowed = Bytes::from(source.as_str());
    let owned = Bytes::try_from(String::from("same")).unwrap();
    assert_eq!(borrowed, owned);
    assert_eq!(borrowed.cmp(&owned), std::cmp::Ordering::Equal);
    let mut set = std::collections::BTreeSet::new();
    set.insert(borrowed);
    set.insert(owned);
    assert_eq!(set.len(), 1);
    let lower = Bytes::from("aaa");
    let upper = Bytes::from("zzz");
    assert!(lower < upper);
}

struct PanicClone {
    clones: Rc<Cell<usize>>,
    drops: Rc<Cell<usize>>,
}
impl Clone for PanicClone {
    fn clone(&self) -> Self {
        let count = self.clones.get();
        self.clones.set(count + 1);
        assert!(count < 2, "third clone panics");
        Self {
            clones: self.clones.clone(),
            drops: self.drops.clone(),
        }
    }
}
impl Drop for PanicClone {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

#[test]
fn inline_vector_clone_cleans_up_on_panic() {
    let clones = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let mut values = InlineVec::<_, 4>::new();
    for _ in 0..3 {
        values
            .push(PanicClone {
                clones: clones.clone(),
                drops: drops.clone(),
            })
            .unwrap();
    }
    assert!(catch_unwind(AssertUnwindSafe(|| values.clone())).is_err());
    assert_eq!(drops.get(), 2);
    assert_eq!(values.len(), 3);
    drop(values);
    assert_eq!(drops.get(), 5);
}

#[test]
fn inline_map_clone_cleans_up_on_panic() {
    let clones = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let mut values = InlineHashMap::<_, _, 4>::new();
    for key in 0..3 {
        values
            .insert(
                key,
                PanicClone {
                    clones: clones.clone(),
                    drops: drops.clone(),
                },
            )
            .unwrap();
    }
    assert!(catch_unwind(AssertUnwindSafe(|| values.clone())).is_err());
    assert_eq!(drops.get(), 2);
    assert_eq!(values.len(), 3);
    drop(values);
    assert_eq!(drops.get(), 5);
}

#[derive(Clone)]
struct Key {
    id: usize,
    panic: Rc<Cell<bool>>,
}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Key {}
impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        assert!(!self.panic.get(), "Hash panics");
        self.id.hash(state);
    }
}
struct DropCount(Rc<Cell<usize>>);
impl Drop for DropCount {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn map_spill_remains_valid_after_hash_panic() {
    let panic = Rc::new(Cell::new(false));
    let drops = Rc::new(Cell::new(0));
    let mut map = InlineHashMap::<_, _, 2>::new();
    for id in 0..2 {
        map.insert(
            Key {
                id,
                panic: panic.clone(),
            },
            DropCount(drops.clone()),
        )
        .unwrap();
    }
    panic.set(true);
    assert!(
        catch_unwind(AssertUnwindSafe(|| map.insert(
            Key {
                id: 2,
                panic: panic.clone()
            },
            DropCount(drops.clone())
        )))
        .is_err()
    );
    panic.set(false);
    // A panic may consume entries, but the remaining prefix must be valid.
    for (_, _) in map.iter() {}
    drop(map);
    assert_eq!(drops.get(), 3);
}

#[test]
fn map_replaces_duplicate_keys_before_spilling() {
    let mut map = InlineHashMap::<_, _, 1>::new();
    map.insert("same", 1).unwrap();
    map.insert("same", 2).unwrap();
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&"same"), Some(&2));
    assert!(!map.is_heap_allocated());
    map.insert("other", 3).unwrap();
    map.insert("same", 4).unwrap();
    assert!(map.is_heap_allocated());
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&"same"), Some(&4));
}

#[cfg(feature = "entities")]
#[test]
fn decoded_text_reuses_borrowed_leaf_and_preserves_mixed_text() {
    use std::borrow::Cow;
    let dom = parse("<div><span><b>plain</b></span></div><p>&amp;<i>Ω</i><!--ignore--><script>&amp;</script></p><em></em>", ParserOptions::default()).unwrap();
    let parser = dom.parser();
    let div = dom
        .query_selector("div")
        .unwrap()
        .next()
        .unwrap()
        .get(parser)
        .unwrap();
    assert!(matches!(
        div.decoded_inner_text(parser),
        Cow::Borrowed("plain")
    ));
    let p = dom
        .query_selector("p")
        .unwrap()
        .next()
        .unwrap()
        .get(parser)
        .unwrap();
    assert_eq!(p.decoded_inner_text(parser), "&Ω&amp;");
    let em = dom
        .query_selector("em")
        .unwrap()
        .next()
        .unwrap()
        .get(parser)
        .unwrap();
    assert!(matches!(em.decoded_inner_text(parser), Cow::Borrowed("")));
}

#[test]
fn nested_has_retains_independent_boundaries() {
    let dom = parse(
        "<ul><li><span><a>x</a></span></li><li><a>y</a></li></ul>",
        ParserOptions::default(),
    )
    .unwrap();
    for (query, count) in [
        ("li:has(span:has(a))", 1),
        ("li:has(ul a)", 0),
        ("li:has(li a)", 0),
        ("ul:has(li > a)", 1),
        ("li:not(:has(span a))", 1),
    ] {
        assert_eq!(dom.query_selector(query).unwrap().count(), count, "{query}");
    }
}

#[test]
fn duplicate_html_attributes_keep_the_first_value() {
    let dom = parse(
        "<p data-x='first' data-x='second'></p>",
        ParserOptions::default(),
    )
    .unwrap();
    let parser = dom.parser();
    let tag = dom
        .query_selector("p")
        .unwrap()
        .next()
        .unwrap()
        .get(parser)
        .unwrap()
        .as_tag()
        .unwrap();
    assert_eq!(
        tag.attributes().get("data-x").flatten().unwrap().as_bytes(),
        b"first"
    );
}
