//! What rust-js refuses, and the one way it says so (ADRs 0262, 0390):
//! "rust-js does not support .. yet", of a feature, a call or a type; and
//! of addressable memory, which JavaScript has none of, that, with why.

use super::recognition::{addresses_memory, is_memory_type};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::{ErrorGuaranteed, Span};

/// What's refused.
#[derive(Clone, Copy)]
pub(crate) enum Refused<'a, 'tcx> {
    /// A feature, as its words say: "a `&mut` to a union's field".
    Feature(&'a str),
    /// A call of a function: "calling `std::env::args`".
    Call(DefId),
    /// Values, constants or statics of a type: "values of type `*const u8`".
    Type(&'a str, Ty<'tcx>),
    /// A feature, and why it's refused: "a component's `Clone`", "React
    /// gives a component no dictionary".
    Because(&'a str, &'a str),
    /// A feature of addressable memory, as its words say: "a raw pointer to a value".
    Memory(&'a str),
}

/// The error that `refused` isn't supported, in its `words`: of
/// addressable memory, with why.
pub(crate) fn refuse<'tcx>(tcx: TyCtxt<'tcx>, span: Span, refused: Refused<'_, 'tcx>) -> ErrorGuaranteed {
    let error = tcx.dcx().struct_span_err(span, words(tcx, refused));
    match of_memory(tcx, refused) {
        true => error.with_note(MEMORY).emit(),
        false => error.emit(),
    }
}

/// Why addressable memory is refused.
const MEMORY: &str = "JavaScript has no memory to address: a raw pointer, `transmute` and `std::ptr` \
                      read or write bytes by where they are (ADR 0390)";

/// What a refusal says: "rust-js does not support .. yet", and of
/// addressable memory, "rust-js does not support addressable memory yet: ..".
pub(crate) fn words<'tcx>(tcx: TyCtxt<'tcx>, refused: Refused<'_, 'tcx>) -> String {
    let what = match refused {
        Refused::Feature(what) | Refused::Because(what, _) | Refused::Memory(what) => what.to_string(),
        Refused::Call(id) => calling(tcx, id),
        Refused::Type(what, ty) => format!("{what} of type `{ty}`"),
    };
    match (of_memory(tcx, refused), refused) {
        (true, _) => format!("rust-js does not support addressable memory yet: {what}"),
        (false, Refused::Because(_, why)) => format!("rust-js does not support {what} yet: {why}"),
        (false, _) => format!("rust-js does not support {what} yet"),
    }
}

/// Whether what's refused is of addressable memory: a call of std's that
/// reads or writes it, a type with a raw pointer or a union in it, or a
/// feature said to be.
fn of_memory<'tcx>(tcx: TyCtxt<'tcx>, refused: Refused<'_, 'tcx>) -> bool {
    match refused {
        Refused::Call(id) => addresses_memory(tcx, id),
        Refused::Type(_, ty) => is_memory_type(tcx, ty),
        Refused::Memory(_) => true,
        Refused::Feature(_) | Refused::Because(..) => false,
    }
}

/// What a refused call is called: "calling `std::env::args`".
pub(crate) fn calling(tcx: TyCtxt<'_>, id: DefId) -> String {
    format!("calling `{}`", tcx.def_path_str(id))
}
