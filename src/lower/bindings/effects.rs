//! What a binding says of what it runs, which nothing of its JS shows:
//! that it can't throw (ADR 0301), that it runs nothing (ADR 0867), or
//! only what it's given (ADR 0898).

use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// `#[rust_js::cannot_throw]`: a binding that can't throw, as reading a
/// data property, React's `ref.current`, can't (ADR 0301).
pub(in crate::lower) fn cannot_throw(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("cannot_throw")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::runs_nothing]`: a binding that runs no code but its own,
/// not even what it's given, as `new AggregateError(errors)` reads its
/// array and not what's in it (ADR 0867): a borrow held while it runs
/// meets no other.
pub(in crate::lower) fn runs_nothing(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("runs_nothing")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::runs_only_given]`: a binding that runs no code but its own
/// and what it's given, as `console.error(..)` and `Math.max(..)` do: a
/// getter, `toString` or a proxy's trap of a value it reads (ADR 0898).
/// Given only what runs nothing, numbers and text, a borrow held while it
/// runs meets no other.
pub(in crate::lower) fn runs_only_given(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("runs_only_given")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}
