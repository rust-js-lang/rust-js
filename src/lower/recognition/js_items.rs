//! The js and react crates' own items that lowering knows by name: a JS
//! collection, a class, a symbol of the registry, a function's `bind`,
//! `call` and `apply`, and a context's `Provider` and `Consumer`.

use super::*;

/// Whether `ty` is a React context's `Provider` or `Consumer`, which
/// `react::provider` gives of a context, and React never changes.
pub(crate) fn is_context_member(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if tcx.crate_name(adt.did().krate).as_str() == "react"
            && matches!(tcx.item_name(adt.did()).as_str(), "Provider" | "Consumer"))
}

/// Of `did`, the js crate's `Set` or `Map`, a JS collection whose own
/// iteration Rust collects, `set.values().collect()`, whether it's the
/// `Set`; `None` of any other (ADR 1052).
pub(in crate::lower) fn js_set_or_map(tcx: TyCtxt<'_>, did: DefId) -> Option<bool> {
    if tcx.crate_name(did.krate).as_str() != "js" {
        return None;
    }
    let name = tcx.item_name(did);
    (name.as_str() == "Set" || name.as_str() == "Map").then(|| name.as_str() == "Set")
}

/// Whether `did` is `react::DependencyList`, a hook's dependencies, a
/// tuple of them its array, `(count, name)`'s `[count, name]`, whose `()`
/// is none, `[]` (ADR 0775).
pub(in crate::lower) fn is_dependency_list(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "react" && tcx.item_name(did).as_str() == "DependencyList"
}

/// Whether `did` is `js::SymbolFor`, the registry's symbol of a key, its JS
/// `Symbol.for(key)` (ADR 0634): the builtins crate's. The one answer, which
/// a constant's value asks too.
pub(in crate::lower) fn is_symbol_for_def(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js" && tcx.item_name(did).as_str() == "SymbolFor"
}

/// Whether `did` is `js::SymbolFor`'s constructor, which is `Symbol.for`
/// as a value, and called as it is where a variable holds it (ADR 0964).
pub(in crate::lower) fn is_symbol_for_constructor(tcx: TyCtxt<'_>, did: DefId) -> bool {
    matches!(tcx.def_kind(did), rustc_hir::def::DefKind::Ctor(..)) && is_symbol_for_def(tcx, did)
}

/// Whether `did` is `js::catch_stack_overflow`, which catches the JS
/// engine's stack overflow (ADR 1140): the builtins crate's.
pub(in crate::lower) fn is_catch_stack_overflow(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js"
        && tcx.item_name(did).as_str() == "catch_stack_overflow"
        && tcx.opt_parent(did).is_some_and(|parent| parent.is_crate_root())
}

/// Whether `did` is `js::any_of`, a type parameter's value as any value
/// (ADR 0909): the builtins crate's.
pub(in crate::lower) fn is_any_of(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js"
        && tcx.item_name(did).as_str() == "any_of"
        && tcx.opt_parent(did).is_some_and(|parent| parent.is_crate_root())
}

/// The name of `did` where it's a function of the js crate's root,
/// `js::new` or `js::unknown`: which the refusal of a constructor held as
/// a value of any shape asks (ADR 1009).
pub(in crate::lower) fn js_root_fn(tcx: TyCtxt<'_>, did: DefId) -> Option<rustc_span::Symbol> {
    (tcx.crate_name(did.krate).as_str() == "js"
        && tcx.def_kind(did) == DefKind::Fn
        && tcx.opt_parent(did).is_some_and(|parent| parent.is_crate_root()))
    .then(|| tcx.item_name(did))
}

/// Whether `id` is `js::function::with_this`, a JS `function` of a
/// closure, whose `arguments` is its own (rd509).
pub(in crate::lower) fn is_with_this(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate).as_str() == "js"
        && tcx.item_name(id).as_str() == "with_this"
        && tcx
            .opt_parent(id)
            .is_some_and(|module| tcx.item_name(module).as_str() == "function")
}

/// Whether `did` is `js::Class`, a constructor function, typed by its
/// signature or not (ADRs 0761, 1009).
pub(in crate::lower) fn is_js_class(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js"
        && tcx.item_name(did).as_str() == "Class"
        && tcx.opt_parent(did).is_some_and(|parent| parent.is_crate_root())
}

/// Whether `ty` is an untyped `js::Class`, `Class<AnyConstructor>`: a
/// class held, a binding's or the crate's, of no constructor's signature,
/// which its constructor's binding makes (ADR 1027, amended).
pub(in crate::lower) fn is_untyped_class(tcx: TyCtxt<'_>, ty: ty::Ty<'_>) -> bool {
    let ty::Adt(class, args) = ty.kind() else {
        return false;
    };
    is_js_class(tcx, class.did())
        && args.types().next().is_some_and(|of| {
            matches!(of.kind(), ty::Adt(of, _)
                if tcx.crate_name(of.did().krate).as_str() == "js" && tcx.item_name(of.did()).as_str() == "AnyConstructor")
        })
}

/// Whether `did` is `Cell::set`, what a constructor sets its `this`'s cell
/// fields with (`classes::set_where_constructed`, rd628).
pub(in crate::lower) fn is_cell_set(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.item_name(did).as_str() == "set"
        && tcx.opt_parent(did).is_some_and(|imp| {
            matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                ty::Adt(adt, _) if adt.did() == std_item(tcx, StdItem::Cell))
        })
}

/// Whether `did` is `js::Class::of`, a function the program writes as the
/// constructor it is (ADR 1009).
pub(in crate::lower) fn is_class_of(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js"
        && tcx.item_name(did).as_str() == "of"
        && tcx.opt_parent(did).is_some_and(|imp| {
            tcx.def_kind(imp) == DefKind::Impl { of_trait: false }
                && matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(adt, _) if is_js_class(tcx, adt.did()))
        })
}

/// Whether `did` is `js::function::bind`, `f.bind(thisArg, ...args)` (ADR
/// 0907, amended).
pub(in crate::lower) fn is_function_bind(tcx: TyCtxt<'_>, did: DefId) -> bool {
    of_function_module(tcx, did, "bind")
}

/// Whether `did` is `js::function::call`, `f.call(thisArg, ...args)`,
/// which is a method's call where `this` is its path's value (rd334).
pub(in crate::lower) fn is_function_call(tcx: TyCtxt<'_>, did: DefId) -> bool {
    of_function_module(tcx, did, "call")
}

/// Whether `did` is `js::function::apply`, `f.apply(thisArg, argArray)`,
/// which calls `f` with the `this` it's given (rd683).
pub(in crate::lower) fn is_function_apply(tcx: TyCtxt<'_>, did: DefId) -> bool {
    of_function_module(tcx, did, "apply")
}

/// Whether `did` is `js::function`'s item `name`.
fn of_function_module(tcx: TyCtxt<'_>, did: DefId, name: &str) -> bool {
    tcx.crate_name(did.krate).as_str() == "js"
        && tcx.item_name(did).as_str() == name
        && tcx.opt_parent(did).is_some_and(|parent| {
            tcx.opt_item_name(parent)
                .is_some_and(|name| name.as_str() == "function")
                && tcx.opt_parent(parent).is_some_and(|root| root.is_crate_root())
        })
}

/// Whether `did` is `js::AnyShape` or `js::AnyFunction`, what a function
/// given as the value it is, its JS function, has (ADRs 0673, 0740).
pub(in crate::lower) fn is_function_value_trait(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js"
        && matches!(tcx.item_name(did).as_str(), "AnyShape" | "AnyFunction")
        && tcx.opt_parent(did).is_some_and(|parent| parent.is_crate_root())
}

/// Whether `did` is `js::SymbolFor::from_value`, a value of any shape read
/// as the registry's symbol (ADR 0634).
pub(in crate::lower) fn is_symbol_for_read(tcx: TyCtxt<'_>, did: DefId) -> bool {
    tcx.crate_name(did.krate).as_str() == "js"
        && tcx.item_name(did).as_str() == "from_value"
        && tcx.opt_parent(did).is_some_and(|imp| {
            tcx.def_kind(imp) == DefKind::Impl { of_trait: false }
                && matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(adt, _) if is_symbol_for_def(tcx, adt.did()))
        })
}
