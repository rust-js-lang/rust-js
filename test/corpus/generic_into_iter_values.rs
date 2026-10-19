// `into_iter()` of a generic `IntoIterator` given a `Vec` is the iterator
// concrete code has, which knows where it is: its `len()`, `size_hint()`
// and clone are a `Vec`'s, read where it's given back (ADR 0475).
fn mk<I: IntoIterator>(i: I) -> I::IntoIter {
    i.into_iter()
}

struct Both<I, J> {
    a: I,
    b: J,
}

fn both<I: IntoIterator, J: IntoIterator>(a: I, b: J) -> Both<I::IntoIter, J::IntoIter> {
    Both { a: a.into_iter(), b: b.into_iter() }
}

fn show<I: IntoIterator<Item = u8>>(items: I) {
    for x in items {
        println!("shown {x}");
    }
}

fn main() {
    let mut it = mk(vec![1, 2, 3]);
    it.next();
    println!("{}", it.len());
    let c = it.clone();
    println!("{:?} {:?} {:?}", it.size_hint(), c.collect::<Vec<_>>(), it.as_slice());
    let pair = both(vec!['a', 'b'], [1u8, 2, 3]);
    println!("{} {}", pair.a.len(), pair.b.len());
    let total: u32 = mk(vec![4u32, 5]).sum();
    for x in mk(vec![6, 7]) {
        println!("{x} {total}");
    }
    show(vec![8, 9]);
}
