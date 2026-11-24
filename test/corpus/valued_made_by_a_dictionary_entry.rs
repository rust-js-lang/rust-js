//@ compile-fail: gives through its impl's dictionary, which generic code
// A write's result an impl's method gives, `fn make(&self) -> fmt::Result`,
// where its trait's signature gives an associated type, `Self::Out`, which
// generic code calling it through the impl's dictionary holds as a value
// (`Form::Valued`, #1410): the method throws its error, as a write's does
// (ADR 0187), and no entry catches it back, so `made_twice(&Bad)` threw
// `$FmtError` where Rust gives `(Err(Error), Err(Error))` (#2559). Refused
// where the impl's dictionary is given, until the entry gives its result
// as a value (#2565).

use std::fmt;

trait Make {
    type Out;
    fn make(&self) -> Self::Out;
}

fn made_twice<M: Make>(m: &M) -> (M::Out, M::Out) {
    (m.make(), m.make())
}

struct Bad;

impl Make for Bad {
    type Out = fmt::Result;
    fn make(&self) -> fmt::Result {
        Err(fmt::Error)
    }
}

fn main() {
    let (a, b) = made_twice(&Bad);
    println!("{a:?} {b:?} {}", a.is_err());
}
