// What `unzip`, `partition`, and a `LinkedList`'s `extend` and `collect`
// built before their source's step panics, as std's do: what `extend` put
// in stays; what the others collected is dropped as the panic unwinds,
// after what's left of their source, `unzip`'s `A`s before its `B`s; of
// the case's own code and of generic code's, given its source or making
// it by a function it's given (#1150, #1161).
use std::collections::LinkedList;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's own iterator, whose `n`th step panics, which prints its
/// drop.
struct Steps<I> {
    items: I,
    step: u32,
    n: u32,
}

impl<I: Iterator> Iterator for Steps<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.step += 1;
        if self.step == self.n {
            panic!("step {}", self.n);
        }
        self.items.next()
    }
}

impl<I> Drop for Steps<I> {
    fn drop(&mut self) {
        println!("drop steps");
    }
}

fn steps(n: u32) -> Steps<std::vec::IntoIter<D>> {
    Steps { items: vec![D(1), D(2), D(3)].into_iter(), step: 0, n }
}

fn pairs(n: u32) -> Steps<std::vec::IntoIter<(D, D)>> {
    Steps { items: vec![(D(1), D(11)), (D(2), D(12)), (D(3), D(13))].into_iter(), step: 0, n }
}

fn caught(f: impl FnOnce()) {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(()) => println!("ended"),
        Err(_) => println!("panicked"),
    }
}

fn unzipped<I: Iterator<Item = (D, D)>>(it: I) {
    let (a, b): (Vec<D>, Vec<D>) = it.unzip();
    println!("{a:?} {b:?}");
}

fn parted<F: Fn() -> I, I: Iterator<Item = D>>(make: F) {
    let (odd, even): (Vec<D>, Vec<D>) = make().partition(|d| d.0 % 2 == 1);
    println!("{odd:?} {even:?}");
}

fn listed<I: Iterator<Item = D>>(it: I) {
    let l: LinkedList<D> = it.collect();
    println!("{l:?}");
}

fn extended<F: Fn() -> I, I: Iterator<Item = D>>(make: F) {
    let mut l = LinkedList::from([D(0)]);
    let r = catch_unwind(AssertUnwindSafe(|| l.extend(make())));
    println!("{} {l:?}", r.is_err());
}

fn main() {
    caught(|| {
        let (a, b): (Vec<D>, Vec<D>) = pairs(3).unzip();
        println!("{a:?} {b:?}");
    });
    caught(|| {
        let (a, b): (Vec<D>, Vec<D>) = vec![(D(1), D(11)), (D(2), D(12))]
            .into_iter()
            .map(|p| if p.0.0 == 2 { panic!("at 2") } else { p })
            .unzip();
        println!("{a:?} {b:?}");
    });
    caught(|| unzipped(pairs(3)));
    caught(|| unzipped(pairs(0)));
    caught(|| parted(|| steps(3)));
    caught(|| listed(steps(3)));
    caught(|| listed(steps(0)));
    extended(|| steps(3));
}
