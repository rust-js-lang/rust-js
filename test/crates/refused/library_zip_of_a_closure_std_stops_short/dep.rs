//! A library whose generic code zips what its consumer gives it, which
//! std's `Zip` of a `TrustedRandomAccess` side stops at its shorter side
//! of, calling the longer side's closure only for the pairs it yields.
pub fn zipped<A: Iterator, B: Iterator>(a: A, b: B) -> Vec<(A::Item, B::Item)> {
    a.zip(b).collect()
}

/// The same zip, through another of the library's functions: its ask is
/// carried to the consumer's call of this one.
pub fn paired<A: Iterator, B: Iterator>(a: A, b: B) -> Vec<(A::Item, B::Item)> {
    zipped(a, b)
}
