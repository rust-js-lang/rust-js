//@ compile-fail: among the items of generic code's
// Write results as the items of an iterator given to generic code that
// reads them as a `Result` of its type parameters, `I: Iterator<Item =
// Result<T, E>>`: generic code reads `{ TAG }` (ADR 0461, amended), its
// caller holds a write's result as `undefined` or its error (ADRs 0187,
// 0462), and the items cross as their caller holds them, so `r.is_ok()`
// read `.TAG` of `undefined` (#2560). Refused at the call, until each item
// is tagged where it crosses (#2566).

use std::fmt;

fn oks<T, E, I: Iterator<Item = Result<T, E>>>(it: I) -> usize {
    it.filter(|r| r.is_ok()).count()
}

fn main() {
    let v: Vec<fmt::Result> = vec![Ok(()), Err(fmt::Error), Ok(())];
    println!("{}", oks(v.into_iter()));
}
