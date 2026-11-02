//! A library that gives `dep`'s `Stack` `()`s, which may look like `None`,
//! as rust_decimal gives arrayvec's `ArrayVec` its digits: it compiles
//! `dep`'s code itself (ADR 0914), the impl's dictionary with its
//! `CAPACITY` of `CAP` too, its initializer from `dep`'s metadata (ADR 0176,
//! amended).
pub fn units() -> String {
    let mut units: dep::Stack<(), 2> = dep::Stack::new();
    let pushed: Vec<bool> = (0..3).map(|_| units.try_push(()).is_ok()).collect();
    format!("{pushed:?} {}", units.len())
}
