// `map` of an `Option` whose value has a destructor: the value moves into
// the function or closure, which drops it where Rust drops it, as
// num-traits' `T::from(n).map(Wrapping)` does in generic code.

use std::num::Wrapping;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Labelled(D);

fn make(n: u32) -> Option<D> {
    if n > 0 { Some(D(n)) } else { None }
}

fn wrapped<T>(value: Option<T>) -> Option<Wrapping<T>> {
    value.map(Wrapping)
}

// Drops `value` where it doesn't keep it: a generic `Option<T>`, whose
// `Some` of an `Option` is boxed (ADR 0051).
fn keep_if<T>(value: Option<T>, keep: bool) -> Option<T> {
    if keep { value } else { None }
}

fn main() {
    let read = make(1).map(|d| d.0 + 10);
    println!("read {:?}", read);
    let kept = make(2).map(Labelled);
    println!("kept {}", kept.as_ref().map_or(0, |l| (l.0).0));
    let none = make(0).map(Labelled);
    println!("none {}", none.is_none());
    let generic = wrapped(make(3));
    println!("generic {}", generic.as_ref().map_or(0, |w| (w.0).0));
    drop(kept);
    println!("{}", keep_if(make(4), false).is_none());
    println!("{}", keep_if(Some(make(5)), false).is_none());
    println!("{}", keep_if(Some(make(0)), false).is_none());
    let nested = keep_if(Some(make(6)), true);
    println!("{}", nested.is_some());
    println!("end");
}
