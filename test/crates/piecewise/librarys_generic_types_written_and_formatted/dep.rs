//! A library's generic types, published as its others are (#1712): each
//! writer writes through a sink, and writes its type parameters by the
//! dictionaries a consumer gives, which its contract says carry `fmtInto`
//! (the impl's `written_into` marks). An `Either` of two, its `Display`
//! and its own `Debug`; a list of a constant capacity, as arrayvec's; a
//! `{:x}` and a `{:p}` forwarded, as thiserror's `Var`; and a derived
//! `Debug`, which the library exports none of and a consumer compiles its
//! own of. The library formats one itself, plainly.
use std::fmt;

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: fmt::Display, R: fmt::Display> fmt::Display for Either<L, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Either::Left(l) => write!(f, "L({l})"),
            Either::Right(r) => write!(f, "R({r})"),
        }
    }
}

impl<L: fmt::Debug, R: fmt::Debug> fmt::Debug for Either<L, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Either::Left(l) => f.debug_tuple("Left").field(l).finish(),
            Either::Right(r) => f.debug_tuple("Right").field(r).finish(),
        }
    }
}

pub struct Small<T, const CAP: usize> {
    items: Vec<T>,
}

impl<T, const CAP: usize> Small<T, CAP> {
    pub fn of(items: Vec<T>) -> Self {
        assert!(items.len() <= CAP);
        Small { items }
    }
}

impl<T: fmt::Display, const CAP: usize> fmt::Display for Small<T, CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{CAP}:")?;
        for item in &self.items {
            write!(f, " {item}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Tagged<T> {
    pub tag: &'static str,
    pub value: T,
}

/// The library's own plain text of one.
pub fn shown(n: u8) -> String {
    format!("{}", Either::<u8, &str>::Left(n))
}

/// `{:x}` and `{:p}` forwarded by a reference, as thiserror's `Var`.
pub struct Var<'a, T: ?Sized>(pub &'a T);

impl<T: fmt::Pointer + ?Sized> fmt::Pointer for Var<'_, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(self.0, formatter)
    }
}

impl<T: fmt::LowerHex + ?Sized> fmt::LowerHex for Var<'_, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self.0, formatter)
    }
}
