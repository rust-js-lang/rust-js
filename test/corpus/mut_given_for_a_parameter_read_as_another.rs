//@ library-differs
// A `&mut` given for a type parameter whose value is read as another its
// bound names, `impl Is<F>` of `F: FnMut()`, `I: Iterator` or `I:
// IntoIterator`: given as that one takes it, the function, the iterator or
// the range itself, not in generic code's box, which `f()` would call and
// `next()` would step (#992). A collection given for an `IntoIterator` is
// its cell, as its dictionary takes it.
trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn twice<F: FnMut()>(f: impl Is<F>) {
    let mut f: F = f.is();
    f();
    f();
}

fn take2<I: Iterator<Item = u32>>(it: impl Is<I>) -> Vec<u32> {
    let it: I = it.is();
    it.take(2).collect()
}

fn total<'a, I: IntoIterator<Item = &'a u32>>(i: impl Is<I>) -> u32 {
    let i: I = i.is();
    i.into_iter().sum()
}

fn bump<'a, I: IntoIterator<Item = &'a mut u32>>(i: impl Is<I>) {
    let i: I = i.is();
    for x in i {
        *x += 1;
    }
}

fn main() {
    let mut count = 0;
    let mut add = || count += 1;
    twice(&mut add);
    println!("{count}");

    let mut range = 1..10u32;
    println!("{:?} {:?}", take2(&mut range), range.next());

    let v = vec![1u32, 2, 3];
    let mut it = v.iter();
    it.next();
    println!("{} {}", total(&mut it), it.count());

    let mut w = vec![1u32, 2];
    bump(&mut w);
    println!("{w:?}");
}
