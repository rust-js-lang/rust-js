// What's made before a call that writes a place runs first: `x + 0` reads
// `x` before `mem::replace` writes it, and a static's `load` before its
// `fetch_add` (ADR 0364).
use std::mem;
use std::sync::atomic::{AtomicU32, Ordering};

static HITS: AtomicU32 = AtomicU32::new(1);

fn main() {
    let mut x = 1u32;
    println!("{} {}", x + 0, mem::replace(&mut x, 5));
    println!("{x}");
    println!("{} {}", HITS.load(Ordering::SeqCst), HITS.fetch_add(10, Ordering::SeqCst));
    println!("{}", HITS.load(Ordering::SeqCst));
}
