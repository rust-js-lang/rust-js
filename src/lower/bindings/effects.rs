//! What a binding says of what it runs, which nothing of its JS shows:
//! that it can't throw (ADR 0301), that it runs nothing (ADR 0867), or
//! only what it's given (ADR 0898), or gives back what it was given (ADR
//! 1022).

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

/// `#[rust_js::iterates_receiver]`: a binding's method of no argument that
/// gives what JS's own iteration of its receiver steps, as a `Set`'s
/// `values()` and `keys()` and a `Map`'s `entries()` do, its
/// `[Symbol.iterator]`: collected, `Array.from(set)` (the user's decision,
/// bw8).
pub(in crate::lower) fn iterates_receiver(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("iterates_receiver")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::gives_back]`: a binding that gives back what the program
/// gave it, React's state, a ref's `current`, a context's value, never an
/// object of JS's own: a struct it gives is the program's, its fields read
/// as plain data (ADR 1022, amended, the user's decision).
pub(in crate::lower) fn gives_back(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("gives_back")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}
