//! What a binding says of what it runs, which nothing of its JS shows:
//! that it can't throw (ADR 0301), that it runs nothing (ADR 0867), or
//! only what it's given (ADR 0898), or gives back what it was given (ADR
//! 1022), or keeps what it's given only in its collection (ADR 1046).

use rustc_hir::def::DefKind;
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

/// `#[rust_js::gives_plain_data]`: a binding that gives fresh plain data, an
/// object JS builds of data properties alone, no getter, as `JSON.parse`
/// and `Object.getOwnPropertyDescriptor` do: a struct it gives, and each it
/// holds, is read as plain data, as one the program builds is (#411; ADR
/// 1022, amended, the user's decision). What it holds JS didn't build is
/// typed as no struct, a descriptor's `value` a `dyn IntoUnknown`.
pub(in crate::lower) fn gives_plain_data(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("gives_plain_data")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::called_constructs]`: a constructor JS's spec makes callable
/// without `new` to the same result, `Error(message)` as `new
/// Error(message)`, Error's and AggregateError's: as a value, the class
/// itself, `Error.bind(null, message, options)`, not an arrow that
/// constructs, which adds a frame to each stack it makes (rd404).
pub(in crate::lower) fn called_constructs(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("called_constructs")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::keeps_in_collection]`: a binding that runs nothing and keeps
/// what it's given only in its collection, the JS object it's called on or,
/// of no receiver, the one it makes and gives, and gives only what that
/// collection holds, as a JS `Map`'s, `Set`'s, `WeakMap`'s and `WeakSet`'s
/// own methods do (ADR 1046): a value a program keeps in one reaches no
/// code but its own, as one it keeps in a `HashMap` doesn't.
pub(in crate::lower) fn keeps_in_collection(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("keeps_in_collection")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// Each `#[rust_js::keeps_in_collection]` that can't say so refused (ADR
/// 1046): one on what isn't a binding, whose body is the program's; on a
/// binding that may run code, which may keep what it's given anywhere, so
/// it says `#[rust_js::runs_nothing]` too; and on one with no collection, a
/// JS object it's called on or, of no receiver, gives.
pub(in crate::lower) fn reject_unkept_collections(
    recognition: &crate::lower::recognition::Recognition<'_, '_>,
) -> bool {
    let tcx = recognition.tcx;
    let mut valid = true;
    for def in tcx.hir_crate_items(()).definitions() {
        let def = def.to_def_id();
        if !keeps_in_collection(tcx, def) {
            continue;
        }
        let function = matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn);
        let refused = match function && super::is_binding(tcx, def) {
            false => Some("it's on a binding, a function JS's code is: this one's body is the program's"),
            true if !crate::lower::analysis::binding_runs_nothing(tcx, def) => Some(
                "a binding that keeps what it's given only in its collection runs no code that could keep it elsewhere: it says `#[rust_js::runs_nothing]` too",
            ),
            true if collection(recognition, def).is_none() => Some(
                "a binding's collection is the JS object it's called on, its `&self`, or, of none, the one it gives, `-> &'static Map<K, V>`: this one has neither",
            ),
            true => None,
        };
        if let Some(why) = refused {
            tcx.dcx().span_err(
                tcx.def_span(def),
                format!("rust-js: `#[rust_js::keeps_in_collection]` can't be said of this (ADR 1046): {why}"),
            );
            valid = false;
        }
    }
    valid
}

/// The collection a `#[rust_js::keeps_in_collection]` binding keeps what
/// it's given in (ADR 1046): the JS object its `&self` is, or, of none, the
/// one it gives.
fn collection<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    def: DefId,
) -> Option<rustc_middle::ty::Ty<'tcx>> {
    let tcx = recognition.tcx;
    let sig = tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(def).instantiate_identity().skip_normalization());
    let of = match super::is_method(tcx, def) {
        true => *sig.inputs().first()?,
        false => sig.output(),
    };
    let held = of.peel_refs();
    recognition.is_js_object(held).then_some(held)
}
