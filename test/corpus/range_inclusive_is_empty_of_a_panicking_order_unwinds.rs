//@ run-fail: boom
// `RangeInclusive::is_empty` runs its `Idx`'s `PartialOrd`, the program's, which
// panics: the guard alive across the call is dropped as the panic unwinds.
// Whether a std call can unwind is read from its MIR as it resolves, not
// from its name, `is_empty` (#1666).
use std::cmp::Ordering;

struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop");
    }
}

#[derive(PartialEq)]
struct K(i32);

impl PartialOrd for K {
    fn partial_cmp(&self, _: &K) -> Option<Ordering> {
        panic!("boom")
    }
}

fn main() {
    let _guard = Loud;
    let r = K(1)..=K(2);
    println!("{}", r.is_empty());
}
