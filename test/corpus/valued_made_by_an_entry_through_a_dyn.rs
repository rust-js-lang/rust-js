//@ compile-fail: gives through its impl's dictionary, which generic code
// A write's result an impl's method gives where its trait's signature gives
// `Self::Out`, made through a `dyn` generic code holds as `dyn Make<Out =
// T>`: the method throws its error, which generic code holding `T` as a
// value doesn't catch, so `made_twice(&Bad)` threw `$FmtError` where Rust
// gives `(Err(Error), Err(Error))`. Where the `dyn` is made, its `Out` is
// the caller's, `fmt::Result`, whatever generic code it's given to, so the
// impl's dictionary is refused there too (#2559), until the entry gives its
// result as a value (#2565).

use std::fmt;

trait Make {
    type Out;
    fn make(&self) -> Self::Out;
}

fn made_twice<T>(d: &dyn Make<Out = T>) -> (T, T) {
    (d.make(), d.make())
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
    println!("{a:?} {b:?}");
}
