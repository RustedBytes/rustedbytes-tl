//! Isolated, single-threaded allocation probe. DOM/fixture setup is excluded.
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

struct Counting;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static REALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
// SAFETY: all allocation/deallocation operations delegate unchanged to System.
// The counters allocate nothing, never panic, and do not access allocated memory.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Relaxed) {
            CALLS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size(), Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Relaxed) {
            CALLS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size(), Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if ACTIVE.load(Relaxed) {
            REALLOCS.fetch_add(1, Relaxed);
            BYTES.fetch_add(size, Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;
fn measure<T>(name: &str, operation: impl FnOnce() -> T) {
    CALLS.store(0, Relaxed);
    REALLOCS.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    ACTIVE.store(true, Relaxed);
    let result = black_box(operation());
    drop(result);
    ACTIVE.store(false, Relaxed);
    println!(
        "{name}: alloc={}, realloc={}, requested_bytes={}",
        CALLS.load(Relaxed),
        REALLOCS.load(Relaxed),
        BYTES.load(Relaxed)
    );
}
fn main() {
    let input = format!("<ul>{}</ul>", "<li><a>plain text</a></li>".repeat(200));
    let dom = tl::parse(&input, tl::ParserOptions::default()).unwrap();
    let node = dom
        .query_selector("ul")
        .unwrap()
        .next()
        .unwrap()
        .get(dom.parser())
        .unwrap();
    let leaf = dom
        .query_selector("a")
        .unwrap()
        .next()
        .unwrap()
        .get(dom.parser())
        .unwrap();
    assert_eq!(dom.query_selector("li:has(a)").unwrap().count(), 200);
    measure("has_200", || {
        dom.query_selector(black_box("li:has(a)")).unwrap().count()
    });
    measure("decoded_text_200", || node.decoded_inner_text(dom.parser()));
    measure("decoded_leaf", || leaf.decoded_inner_text(dom.parser()));
}
