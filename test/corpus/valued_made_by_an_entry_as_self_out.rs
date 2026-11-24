//@ compile-fail: gives through its impl's dictionary, which generic code
// A write's result an impl's method gives as its trait's `Self::Out`, an
// `io::Result<()>` of a generic impl, which its method is compiled as, so
// it throws its error, and generic code calling it through the impl's
// dictionary holds `M::Out` as a value: refused where the dictionary is
// given (#2559), until the entry gives its result as a value (#2565).

use std::io;

trait Make {
    type Out;
    fn make(&self) -> Self::Out;
}

fn made_twice<M: Make>(m: &M) -> (M::Out, M::Out) {
    (m.make(), m.make())
}

struct Bad<T>(T);

impl<T> Make for Bad<T> {
    type Out = io::Result<()>;
    fn make(&self) -> Self::Out {
        Err(io::Error::other("no"))
    }
}

fn main() {
    let (a, b) = made_twice(&Bad(1u8));
    println!("{} {}", a.is_err(), b.is_err());
}
