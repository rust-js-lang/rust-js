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
    let mut error = tcx.dcx().struct_span_err(span, words(tcx, refused));
    if of_memory(tcx, refused) {
        error = error.with_note(MEMORY);
    }
    if let Refused::Call(id) = refused
        && let Some(why) = uncompiled(tcx, id)
    {
        error = error.with_note(format!(
            "its crate is its types alone: rust-js couldn't compile it (ADR 0415): {why}"
        ));
    }
    error.emit()
}

/// Why rust-js didn't compile the crate of `id`, a crate of crates.io's it
/// refused, which rustc checked, its types alone (ADR 0415), as written
/// beside its metadata.
fn uncompiled(tcx: TyCtxt<'_>, id: DefId) -> Option<String> {
    if id.is_local() {
        return None;
    }
    let metadata = tcx.used_crate_source(id.krate).rmeta.clone()?;
    std::fs::read_to_string(crate::library::refusal(&metadata)).ok()
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
