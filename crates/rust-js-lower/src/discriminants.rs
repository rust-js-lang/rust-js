//! A fieldless enum's discriminants: each variant's, and a discriminant's
//! value as its type reads it, signed or unsigned, and as JS has it. Split
//! from lib.rs by concept, as a file is at most 1,500 lines.

use rust_js_program::js::Expr;
use rustc_middle::ty::{self, TyCtxt};

use crate::bindings;

/// A fieldless enum's variants, each its name and its discriminant, in the
/// order they're declared: `Red = 1` is `("Red", 1)`.
pub(crate) fn of<'tcx>(tcx: TyCtxt<'tcx>, adt: ty::AdtDef<'tcx>) -> Vec<(String, Discr)> {
    adt.discriminants(tcx)
        .map(|(index, d)| (bindings::variant_name(tcx, adt.variant(index)), Discr::of(tcx, d)))
        .collect()
}

/// A discriminant's value, as its type reads it, the one answer of it: a
/// signed `repr`'s sign-extended, `-1` of an `enum E { A = -1 }`; an
/// unsigned one's as it is, a `u128`'s past `i128::MAX` too, which an
/// `i128` would wrap negative (#1131). One enum's are all one kind, so their
/// order is their values'.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Discr {
    Signed(i128),
    Unsigned(u128),
}

impl Discr {
    pub fn of<'tcx>(tcx: TyCtxt<'tcx>, d: ty::util::Discr<'tcx>) -> Discr {
        match d.ty.is_signed() {
            true => {
                let bits = d.ty.primitive_size(tcx).bits();
                Discr::Signed(((d.val as i128) << (128 - bits)) >> (128 - bits))
            }
            false => Discr::Unsigned(d.val),
        }
    }

    /// Its value as JS has it: a BigInt where `big`, a 64-bit or wider
    /// `repr`'s, else a number.
    pub fn expr(self, big: bool) -> Expr {
        match (self, big) {
            (Discr::Signed(n), true) => Expr::bigint(n),
            (Discr::Unsigned(n), true) => Expr::biguint(n),
            (Discr::Signed(n), false) => Expr::int(n),
            (Discr::Unsigned(n), false) => Expr::int(n as i128),
        }
    }

    /// Whether it's in a type's range, `lo` to `hi`.
    pub(crate) fn fits(self, lo: i128, hi: u128) -> bool {
        match self {
            Discr::Signed(n) => lo <= n && (n < 0 || n as u128 <= hi),
            Discr::Unsigned(n) => n <= hi && (lo <= 0 || i128::try_from(n).is_ok_and(|n| lo <= n)),
        }
    }

    /// Its value, where an `i128` holds it.
    pub(crate) fn as_i128(self) -> Option<i128> {
        match self {
            Discr::Signed(n) => Some(n),
            Discr::Unsigned(n) => i128::try_from(n).ok(),
        }
    }
}
