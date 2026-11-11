// What a compiled struct asks the hint of, a `Fuse`'s or a `Peekable`'s
// source, held by the crate's own struct, which makes that kind explicit
// wherever it's made (ADR 0542): a `take(n)` of a held `Repeat` or of a
// held `Cycle`, each `$sizeHint` of what it keeps (#1109).
struct Holder {
    it: std::iter::Repeat<u32>,
}

struct Cycled {
    it: std::iter::Cycle<std::ops::Range<u32>>,
}

fn main() {
    let held = Holder {
        it: std::iter::repeat(7),
    };
    let fused = held.it.take(3).fuse();
    println!("{:?}", fused.size_hint());
    let cycled = Cycled { it: (0..3u32).cycle() };
    let taken = cycled.it.take(5).fuse();
    println!("{:?}", taken.size_hint());
}
