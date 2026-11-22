//@ run-fail: iterator is infinite
// A `Repeat`'s `count`, which panics as it has no end, names where it's
// called, as its impl tracks its caller: generic code's `it.count()`, as
// rustc's monomorphized call gives it, held while it runs; in the build of
// std's iterator structs, std's compiled `Repeat::count`, reached through
// its step table, given it by the runtime (#2324).
fn total<I: Iterator>(it: I) -> usize {
    it.count()
}

fn main() {
    println!("{}", total(std::iter::repeat(1).take(3)));
    println!("{}", total(std::iter::repeat(1)));
}
