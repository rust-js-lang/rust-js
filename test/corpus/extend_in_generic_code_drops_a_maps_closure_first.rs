// Generic code's `Vec::extend` of a map of what its caller gives: std
// steps a `TrustedLen` one by its fold, `extend_trusted`, which moves the
// map's closure into it, so as the closure panics the closure's capture
// drops before the source's rest; another by `next`. Whether it is, its
// caller's, `ITrustedLen` (#1805).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn grow<I: Iterator<Item = D>>(src: I) -> usize {
    let guard = D(91);
    let mut calls = 0;
    let mut grown = Vec::new();
    grown.extend(src.map(move |d| {
        let _held = &guard;
        calls += 1;
        if calls == 3 {
            panic!("map");
        }
        D(d.0 + 100)
    }));
    grown.len()
}

fn main() {
    let trusted = std::panic::catch_unwind(|| grow(vec![D(1), D(2), D(3), D(4), D(5)].into_iter()));
    println!("trusted {}", trusted.is_err());
    let filtered = std::panic::catch_unwind(|| grow(vec![D(6), D(7), D(8), D(9)].into_iter().filter(|d| d.0 > 0)));
    println!("filtered {}", filtered.is_err());
}
