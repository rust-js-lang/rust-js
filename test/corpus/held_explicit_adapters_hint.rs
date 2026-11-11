// What a compiled struct asks the hint of, a `Fuse`'s or a `Peekable`'s
// source, held by the crate's own struct, which makes that kind explicit
// wherever it's made (ADR 0542): a `take(n)` of a held `Repeat` or of a
// held `Cycle`, each `$sizeHint` of what it keeps (#1109); and a held
// `Take` of a type parameter's iterator, whose hint its caller's knows,
// and of a `once`, an array made in place, which no rule but the explicit
// adapter's knows the hint of.
struct Holder {
    it: std::iter::Repeat<u32>,
}

struct Cycled {
    it: std::iter::Cycle<std::ops::Range<u32>>,
}

struct Single {
    it: std::iter::Take<std::iter::Once<u32>>,
}

struct Window<I> {
    rest: std::iter::Take<I>,
}

fn first_two<I: Iterator>(items: I) -> (usize, Option<usize>) {
    let window = Window { rest: items.take(2) };
    window.rest.fuse().size_hint()
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
    let single = Single {
        it: std::iter::once(9).take(1),
    };
    println!("{:?}", single.it.size_hint());
    println!("{:?}", first_two(vec![1, 2, 3].into_iter()));
    println!("{:?}", first_two([1u8].iter()));
    println!("{:?}", first_two(0..10));
}
