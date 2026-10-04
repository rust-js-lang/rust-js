// `map` of an `Option` or a `Result` whose value has a destructor: the value moves into
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

fn checked(n: u32) -> Result<D, String> {
    if n < 9 { Ok(D(n + 20)) } else { Err(format!("{n} is too big")) }
}

fn wrapped_result<T, E>(value: Result<T, E>) -> Result<Wrapping<T>, E> {
    value.map(Wrapping)
}

fn discard_result<T, E>(value: Result<T, E>, keep: bool) -> bool {
    if keep {
        drop(value);
        true
    } else {
        false
    }
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
    let ok = checked(1).map(Labelled);
    println!("ok {}", ok.as_ref().map_or(0, |l| (l.0).0));
    match checked(12).map(Labelled) {
        Ok(l) => println!("ok {}", (l.0).0),
        Err(e) => println!("err {e}"),
    }
    let generic = wrapped_result(checked(2));
    println!("generic {}", generic.as_ref().map_or(0, |w| (w.0).0));
    println!("{} {}", discard_result(checked(3), false), discard_result(checked(4), true));
    println!("end");
}
