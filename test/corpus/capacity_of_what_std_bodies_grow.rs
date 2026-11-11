// What a body `@rust-js/std` compiles grows, `BufRead::read_until`'s
// default's `extend_from_slice`, keeps the capacity std gives it, as every
// consumer's `capacity()` may ask: std's manifest says its bodies keep
// capacities, which a consumer trusts, and they were built keeping none,
// so `read_until` into an empty `Vec` gave `4 4` where native Rust gives
// `4 8` (#1339). And `split`'s items, made by the same default.
use std::io::BufRead;

fn main() {
    let mut v = Vec::new();
    (&b"xyz\n"[..]).read_until(b'\n', &mut v).unwrap();
    println!("{} {}", v.len(), v.capacity());
    let mut longer = Vec::with_capacity(2);
    (&b"abcdefghij"[..]).read_until(b'\n', &mut longer).unwrap();
    println!("{} {}", longer.len(), longer.capacity());
    for item in BufRead::split(&b"ab,cdefg,h"[..], b',') {
        let item = item.unwrap();
        println!("{} {}", item.len(), item.capacity());
    }
}
