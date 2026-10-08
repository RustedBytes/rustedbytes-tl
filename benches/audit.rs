use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn fixture() -> String {
    let mut input = String::from("<ul>");
    for _ in 0..200 {
        input.push_str("<li><a>plain text</a></li>");
    }
    input.push_str("</ul>");
    input
}

fn benchmarks(c: &mut Criterion) {
    let input = fixture();
    let dom = tl::parse(&input, tl::ParserOptions::default()).unwrap();
    let node = dom
        .query_selector("ul")
        .unwrap()
        .next()
        .unwrap()
        .get(dom.parser())
        .unwrap();
    assert_eq!(dom.query_selector("li:has(a)").unwrap().count(), 200);
    assert_eq!(
        node.decoded_inner_text(dom.parser()),
        "plain text".repeat(200)
    );
    c.bench_function("audit/has_200", |b| {
        b.iter(|| black_box(dom.query_selector(black_box("li:has(a)")).unwrap().count()))
    });
    c.bench_function("audit/decoded_text_200", |b| {
        b.iter(|| black_box(node.decoded_inner_text(black_box(dom.parser()))))
    });
}
criterion_group!(benches, benchmarks);
criterion_main!(benches);
