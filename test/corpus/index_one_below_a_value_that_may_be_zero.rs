// `p[i - 1]` under `i < p.len()` alone: `i - 1` of `i = 0` wraps, past
// every length, so the index's check is made. The MIR intervals' relation
// `i - 1 < len` of `i < len` holds only where `i` is known at least one;
// without that, the JS read `p[4294967295]`, `undefined`, where native
// Rust panics (#1672 M3, ADR 1137).
use std::panic::catch_unwind;

fn before(p: &[u32], i: usize) -> u32 {
    if i < p.len() { p[i - 1] } else { 0 }
}

fn main() {
    println!("{}", before(&[1, 2], std::hint::black_box(1)));
    let r = catch_unwind(|| before(&[1, 2], std::hint::black_box(0)));
    println!("{}", r.is_err());
}
