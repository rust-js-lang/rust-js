//@ compile-fail: gives through its impl's dictionary, which generic code
// A write's result an impl's method gives, `fn is(&self) -> fmt::Result`,
// where its trait's signature gives the trait's type parameter, `-> T`,
// and generic code's bound names it as its own, `W: Is<T>`, so it holds
// what the method gives as a value, where the method throws its error (ADR
// 0187): `twice(&Writes)` would throw `$FmtError` where Rust gives
// `(Err(Error), Err(Error))` (#2559). Where the bound names the write's
// result, `impl Is<fmt::Result>`, its generic code calls it as a write
// (#1634) and it compiles (`write_result_through_a_dictionary`); here the
// call refuses the dictionary, until the entry gives its result as a value
// (#2565).

use std::fmt;

trait Is<T> {
    fn is(&self) -> T;
}

fn twice<T, W: Is<T>>(w: &W) -> (T, T) {
    (w.is(), w.is())
}

struct Writes;

impl Is<fmt::Result> for Writes {
    fn is(&self) -> fmt::Result {
        Err(fmt::Error)
    }
}

fn main() {
    let (a, b) = twice(&Writes);
    println!("{a:?} {b:?}");
}
