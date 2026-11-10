//! A library's generic `eq` of what its consumers give, itertools' `equal`:
//! std's `SpecIterEq` compares two `TrustedLen` sides' lengths first,
//! which each consumer's `LengthFirst` says of what it gives (#1182).
pub fn equal<I, J>(a: I, b: J) -> bool
where
    I: IntoIterator,
    J: IntoIterator,
    I::Item: PartialEq<J::Item>,
{
    a.into_iter().eq(b)
}
