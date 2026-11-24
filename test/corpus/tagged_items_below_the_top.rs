//@ compile-fail: among the items of generic code's
// Write results held inside the items of an iterator given to generic code,
// `Item = Option<Result<T, E>>`, which reads them as `Result`s of its type
// parameters, `{ TAG }`, where its caller holds `undefined` or the error:
// `somes` counted none where Rust counts one (#2560). Refused at the call,
// until each item is tagged where it crosses (#2566).

use std::fmt;

fn somes<T, E, I: IntoIterator<Item = Option<Result<T, E>>>>(it: I) -> usize {
    it.into_iter().filter(|r| matches!(r, Some(Ok(_)))).count()
}

fn main() {
    let v: Vec<Option<fmt::Result>> = vec![Some(Ok(())), Some(Err(fmt::Error)), None];
    println!("{}", somes(v));
}
