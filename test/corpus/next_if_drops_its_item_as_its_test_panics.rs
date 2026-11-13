// `Peekable::next_if` whose test panics: the item it took to ask the test
// of is dropped as the panic unwinds, before what's left of the iterator,
// as std's holds it in a local then. It leaked (#1648). Its item taken, or
// kept as what it peeked at, as before; of items that may look like `None`
// too.
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug)]
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut it = vec![Loud(4), Loud(5), Loud(6)].into_iter().peekable();
        let got = it.next_if(|l| {
            assert!(l.0 != 4, "predicate");
            true
        });
        got.map(|l| l.0)
    }));
    println!("{}", r.is_err());

    let mut it = vec![Loud(1), Loud(2)].into_iter().peekable();
    let taken = it.next_if(|l| l.0 == 1);
    println!("taken {taken:?}");
    let kept = it.next_if(|l| l.0 == 9);
    println!("kept {kept:?} then {:?}", it.next());

    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut it = vec![Some(Loud(7)), None].into_iter().peekable();
        let got = it.next_if(|l| {
            assert!(l.is_none(), "an option");
            true
        });
        got.is_some()
    }));
    println!("{}", r.is_err());
}
