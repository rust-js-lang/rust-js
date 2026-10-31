//! What `#[rust_js::nullable]` says, of a field, a function's result and
//! its parameters, and a thread-local: that a `None` is `null` there (ADRs
//! 0196, 0275, 0586).

use crate::lower::recognition::{StdItem, is_std_def};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, FieldDef, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// Whether a field of an `Option` is declared to take `null` too,
/// `#[rust_js::nullable]` (ADR 0196).
pub(in crate::lower) fn is_nullable(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(field.did, &path).next().is_some()
}

/// Whether a nullable field is absent until set, `#[rust_js::optional]`, `field?: T | null` (ADR 0810).
pub(in crate::lower) fn is_optional(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("optional")];
    tcx.get_attrs_by_path(field.did, &path).next().is_some()
}

/// Whether a field's `None` is `null` from where it's made on, so its read is `null` or a value: a
/// nullable one's, not an optional one's, which a literal's `None` leaves out (ADR 0810, amended).
pub(in crate::lower) fn null_where_made(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    is_nullable(tcx, field) && !is_optional(tcx, field)
}

/// The `Option` a `#[rust_js::nullable]` field of type `ty` holds: itself,
/// or a `Cell`'s, whose `None` is `null` in JS (ADRs 0196, 0772), or a
/// `RefCell`'s, a field's the value it holds (ADRs 0362, 0863). `None` of
/// any other type.
pub(in crate::lower) fn nullable_option<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let option = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    let cell = |adt: ty::AdtDef<'_>, item| adt.did() == crate::lower::recognition::std_item(tcx, item);
    match ty.kind() {
        _ if option(ty) => Some(ty),
        ty::Adt(adt, args)
            if (cell(*adt, crate::lower::recognition::StdItem::Cell)
                || cell(*adt, crate::lower::recognition::StdItem::RefCell))
                && option(args.type_at(0)) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// Whether a `#[rust_js::nullable]` field of type `ty`, a generic struct's,
/// is of a type parameter's value, itself or a `Cell`'s or a `RefCell`'s,
/// `current: RefCell<T>`: an `Option` where its struct is one of an
/// `Option`, `StackCursor<Option<Rc<HiddenContext>>>`, whose `None` is then
/// `null` as `nullable_option`'s is; as it is of any other (ADR 0586,
/// amended).
pub(in crate::lower) fn of_type_parameter(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let cell = |adt: ty::AdtDef<'_>, item| adt.did() == crate::lower::recognition::std_item(tcx, item);
    match ty.kind() {
        ty::Param(_) => true,
        ty::Adt(adt, args)
            if cell(*adt, crate::lower::recognition::StdItem::Cell)
                || cell(*adt, crate::lower::recognition::StdItem::RefCell) =>
        {
            matches!(args.type_at(0).kind(), ty::Param(_))
        }
        _ => false,
    }
}

/// The `Option` each item of a `#[rust_js::nullable]` field of type `ty`
/// is, a `Vec<Option<T>>`'s or a `Cell`'s or a `RefCell`'s of one, whose
/// `None` is `null` in JS, upstream's `createLaneMap(null)` (ADR 0586,
/// amended). `None` of any other type.
pub(in crate::lower) fn nullable_items<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let std = |item| adt.did() == crate::lower::recognition::std_item(tcx, item);
    if std(crate::lower::recognition::StdItem::Cell) || std(crate::lower::recognition::StdItem::RefCell) {
        return nullable_items(tcx, args.type_at(0));
    }
    let item = args.types().next()?;
    (std(crate::lower::recognition::StdItem::Vec) && nullable_option(tcx, item) == Some(item)).then_some(item)
}

/// The parameters a binding names `#[rust_js::nullable(..)]`, each
/// `T | null` as a nullable field is (ADR 0275). On the function, since a
/// parameter's own attributes are kept neither in an `extern` block nor
/// in another crate's metadata.
pub(in crate::lower) fn nullable_params(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<Symbol> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(def_id, &path)
        .filter_map(|attr| attr.meta_item_list())
        .flatten()
        .filter_map(|item| item.ident().map(|ident| ident.name))
        .collect()
}

/// Each binding of `owner`'s parameters, a closure's, that its parameter's
/// own `#[rust_js::nullable(..)]` names, `T | null` as a function's own
/// nullable parameter is (ADR 0861, amended): `function::with_this`'s
/// tuple, `#[rust_js::nullable(react_name, target_inst)] (react_name, ..):
/// (..)`, as a closure takes no attribute of its own. Its name and its
/// pattern's span, which MIR's debug info has. Read from the body, as a
/// parameter's `#[rust_js::name]` is.
pub(in crate::lower) fn nullable_bindings(
    tcx: TyCtxt<'_>,
    owner: rustc_span::def_id::LocalDefId,
) -> Vec<(Symbol, rustc_span::Span)> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    let params = tcx.hir_maybe_body_owned_by(owner).map_or(&[][..], |body| body.params);
    let mut found = Vec::new();
    for param in params {
        let named: Vec<Symbol> = (tcx.hir_attrs(param.hir_id).iter())
            .filter(|attr| attr.path_matches(&path))
            .filter_map(|attr| attr.meta_item_list())
            .flatten()
            .filter_map(|item| item.ident().map(|ident| ident.name))
            .collect();
        if named.is_empty() {
            continue;
        }
        param.pat.walk_always(|pat| {
            if let rustc_hir::PatKind::Binding(_, _, ident, _) = pat.kind
                && named.contains(&ident.name)
            {
                found.push((ident.name, pat.span));
            }
        });
    }
    found
}

/// Whether item `item` of `owner`'s parameter `param`, a closure's tuple,
/// `(name, inst)`'s `inst` of `#[rust_js::nullable(name, inst)]`, is a
/// binding that attribute names: what the closure's body binds it of, the
/// tuple's item, is `null` or a value, as it's declared (ADR 0861,
/// amended), not a `None` the body sets.
pub(in crate::lower) fn nullable_tuple_item(
    tcx: TyCtxt<'_>,
    owner: rustc_span::def_id::LocalDefId,
    param: usize,
    item: usize,
) -> bool {
    let Some(body) = tcx.hir_maybe_body_owned_by(owner) else {
        return false;
    };
    let Some(param) = body.params.get(param) else {
        return false;
    };
    let rustc_hir::PatKind::Tuple(items, dots) = param.pat.kind else {
        return false;
    };
    if dots.as_opt_usize().is_some() {
        return false;
    }
    let Some(rustc_hir::Pat {
        kind: rustc_hir::PatKind::Binding(_, _, ident, None),
        ..
    }) = items.get(item)
    else {
        return false;
    };
    nullable_bindings(tcx, owner)
        .iter()
        .any(|(name, _)| *name == ident.name)
}

/// What's wrong with a `#[rust_js::nullable]` on a parameter of `owner`'s:
/// on a closure's, each name it gives is a binding of that parameter that's
/// an `Option`; a function's says so on the function, as a binding's does,
/// whose parameter's attributes no crate's metadata keeps (ADR 0861,
/// amended).
pub(in crate::lower) fn nullable_parameters_refused(
    tcx: TyCtxt<'_>,
    owner: rustc_span::def_id::LocalDefId,
) -> Vec<(rustc_span::Span, String)> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    let Some(body) = tcx.hir_maybe_body_owned_by(owner) else {
        return Vec::new();
    };
    let closure = tcx.is_closure_like(owner.to_def_id());
    let mut refused = Vec::new();
    for param in body.params {
        for attr in tcx
            .hir_attrs(param.hir_id)
            .iter()
            .filter(|attr| attr.path_matches(&path))
        {
            if !closure {
                let name = match param.pat.kind {
                    rustc_hir::PatKind::Binding(_, _, ident, None) => ident.name.to_string(),
                    _ => "..".to_string(),
                };
                let message = format!(
                    "rust-js: a function's parameter is `#[rust_js::nullable({name})]` on the function, as a binding's is; a closure's is on its parameter"
                );
                refused.push((attr.span(), message));
                continue;
            }
            let names = (attr.meta_item_list().into_iter().flatten()).filter_map(|item| item.ident());
            for ident in names {
                let mut option = false;
                param.pat.walk_always(|pat| {
                    if let rustc_hir::PatKind::Binding(_, hir_id, bound, _) = pat.kind
                        && bound.name == ident.name
                    {
                        let ty = tcx.typeck(owner).node_type(hir_id);
                        option |= matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
                    }
                });
                if !option {
                    let message = format!(
                        "rust-js: `#[rust_js::nullable({})]` names no binding of this parameter that's an `Option`",
                        ident.name
                    );
                    refused.push((attr.span(), message));
                }
            }
        }
    }
    refused
}

/// Whether a function says its result's `None` is `null`,
/// `#[rust_js::nullable]` alone on it, TypeScript's `T | null`, as a JS
/// caller of it tests `!== null`: React's scheduler's `peek(queue)`.
pub(in crate::lower) fn nullable_result(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    nullable_alone(tcx, def_id)
}

/// Whether a binding says its result's `None` is `undefined`, never
/// `null`, `#[rust_js::optional]` alone on it, TypeScript's `T |
/// undefined`: `window.event` (ADR 0298, amended). Of the crate's own
/// functions it says nothing: their `None` is `undefined` where they make
/// it, but what they return may be a value JS gave.
pub(in crate::lower) fn optional_result(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("optional")];
    super::is_binding(tcx, def_id) && tcx.get_attrs_by_path(def_id, &path).any(|attr| attr.is_word())
}

/// Why a function's `#[rust_js::nullable]` is refused, where its result
/// isn't an `Option`. Where the result holds one, an array's or a tuple's,
/// its `None`s are `null` where they're written `js::null()` (ADR 0935,
/// amended): what's returned holds what's stored in it, as a plain place
/// does.
pub(super) fn refused_result(tcx: TyCtxt<'_>, def_id: DefId) -> Option<&'static str> {
    let output = tcx.fn_sig(def_id).skip_binder().skip_binder().output();
    let is_option = |ty: Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    if !nullable_result(tcx, def_id) || is_option(output) {
        return None;
    }
    let holds_option = output.walk().any(|arg| arg.as_type().is_some_and(is_option));
    Some(if holds_option {
        "rust-js: `#[rust_js::nullable]` of a function is of its result, which isn't an `Option`; a `None` in it that's `null` is written `js::null()`"
    } else {
        "rust-js: `#[rust_js::nullable]` of a function is of its result, which isn't an `Option`"
    })
}

/// Whether `key`, a `thread_local!`'s, says its cell's `None` is `null`,
/// `#[rust_js::nullable]` alone on its static (ADR 0586, amended): its
/// module's variable, `null` where it's made and set, as a nullable
/// field's is, `Fiber | null` to TypeScript, as React's `let current:
/// Fiber | null = null` is. Or a `const`'s, of an `Option`: `export const
/// NoTransition = null`.
pub(in crate::lower) fn nullable_local(tcx: TyCtxt<'_>, key: DefId) -> bool {
    nullable_alone(tcx, key)
}

/// Whether `key`, a `#[rust_js::nullable]` thread-local's, holds an
/// `Option` itself, no cell of one: a `const` of its module, `null` where
/// it's made `None`, `export const callComponentInDEV = __DEV__ ? .. :
/// null`, as React's is (ADR 0586, amended).
pub(in crate::lower) fn nullable_local_option(tcx: TyCtxt<'_>, key: DefId) -> bool {
    let option = |ty: Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    nullable_local(tcx, key)
        && matches!(tcx.type_of(key).instantiate_identity().skip_normalization().kind(),
            ty::Adt(local_key, args) if crate::lower::recognition::is_std_def(tcx, local_key.did(), crate::lower::recognition::StdItem::LocalKey)
                && option(args.type_at(0)))
}

/// What's wrong with `#[rust_js::nullable]` on `def`, a `const`, or a
/// `thread_local!`'s key, which is one: a thread-local's is of an `Option`,
/// or a `Cell` or a `RefCell` of one, or of a generic type made of one,
/// `StackCursor<Option<T>>`, whose generic constructor's given `None` is
/// `null`; a `const`'s is of an `Option` (ADR 0586, amended).
pub(in crate::lower) fn nullable_const_refused(tcx: TyCtxt<'_>, def: DefId) -> Option<&'static str> {
    if !nullable_local(tcx, def) {
        return None;
    }
    let option = |ty: Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    let ty = tcx.type_of(def).instantiate_identity().skip_normalization();
    match ty.kind() {
        ty::Adt(key, args) if is_std_def(tcx, key.did(), StdItem::LocalKey) => {
            let held = args.type_at(0);
            let of_option = nullable_option(tcx, held).is_some() || made_of_options(tcx, held);
            (!of_option).then_some(
                "rust-js: `#[rust_js::nullable]` of a thread-local is of one of an `Option`, or a `Cell` or a `RefCell` of one, or of a generic type made of one, `StackCursor<Option<T>>`, whose `None` is then `null`; this one isn't",
            )
        }
        _ => (!option(ty)).then_some(
            "rust-js: `#[rust_js::nullable]` of a `const` is of one that's an `Option`, whose `None` is then `null`; this one isn't",
        ),
    }
}

/// Whether `held`, what a `#[rust_js::nullable]` thread-local holds, is
/// of a generic type one of whose type parameters is an `Option`,
/// `StackCursor<Option<Rc<HiddenContext>>>`, as a generic constructor
/// makes it, `create_cursor(None)`: each `Option` its initializer's call
/// is given for a type parameter's value is `null` for `None` (ADR 0586,
/// amended).
fn made_of_options(tcx: TyCtxt<'_>, held: Ty<'_>) -> bool {
    let option = |ty: Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    matches!(held.kind(), ty::Adt(adt, args) if !tcx.is_lang_item(adt.did(), LangItem::Option) && args.types().any(option))
}

/// Whether what `body` gives has its `None` `null`, as it writes it (ADR
/// 0578): a `#[rust_js::nullable]` function's result; and a nullable
/// thread-local's `init`, of one that holds the `Option` itself
/// (`nullable_local_option`), the value it's made, `__DEV__ ? f : null`.
pub(in crate::lower) fn null_result(tcx: TyCtxt<'_>, body: DefId) -> bool {
    nullable_result(tcx, body) || tcx.opt_parent(body).is_some_and(|key| nullable_local_option(tcx, key))
}

/// Whether `body`, a thread-local's `init`, is a `#[rust_js::nullable]`
/// one's (`nullable_local`), whose `None`s are `null` where it's made.
pub(in crate::lower) fn nullable_init(tcx: TyCtxt<'_>, body: DefId) -> bool {
    tcx.opt_parent(body).is_some_and(|key| nullable_local(tcx, key))
}

/// `#[rust_js::nullable]` alone on `id`, of no parameter's name.
fn nullable_alone(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(id, &path).any(|attr| attr.is_word())
}

/// Is this `js::null()`, written as the `null` it gives, `link_name =
/// "null"` (ADR 0935, amended)? A constant written of it is its
/// initializer's, not rustc's value, whose `None` would be left out.
pub(in crate::lower) fn is_js_null(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    super::is_binding(tcx, def_id) && matches!(super::js_form(tcx, def_id), super::JsForm::Null)
}

/// The type parameters of a collection, a struct `#[rust_js::nullable(..)]`
/// names, whose `None` it holds as `null`: a `js::Map`'s `V`, TypeScript's
/// `Map<K, V | null>` (ADR 0884, amended, the user's decision). Their
/// indexes.
pub(in crate::lower) fn nullable_values(tcx: TyCtxt<'_>, adt: DefId) -> Vec<u32> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    let names: Vec<Symbol> = (tcx.get_attrs_by_path(adt, &path))
        .filter_map(|attr| attr.meta_item_list())
        .flatten()
        .filter_map(|item| item.ident().map(|ident| ident.name))
        .collect();
    (tcx.generics_of(adt).own_params.iter())
        .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Type { .. }) && names.contains(&param.name))
        .map(|param| param.index)
        .collect()
}

/// The type parameters of binding `def_id` that are a collection's values
/// it holds `None` of as `null` (`nullable_values`), where its signature
/// names the collection: `V` of `Map<K, V>::set(&self, key, value: V)`
/// and of `map::new(..) -> &Map<K, V>`.
fn nullable_type_params(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<u32> {
    if !super::is_binding(tcx, def_id) {
        return Vec::new();
    }
    let sig = tcx.fn_sig(def_id).skip_binder().skip_binder();
    let mut found = Vec::new();
    for ty in sig.inputs_and_output.iter() {
        for part in ty.walk() {
            if let Some(ty::Adt(adt, args)) = part.as_type().map(|t| *t.kind()) {
                for at in nullable_values(tcx, adt.did()) {
                    if let ty::Param(p) = args.type_at(at as usize).kind()
                        && !found.contains(&p.index)
                    {
                        found.push(p.index);
                    }
                }
            }
        }
    }
    found
}

/// Whether binding `def_id`'s `i`th parameter takes `null` for `None`:
/// one `#[rust_js::nullable(..)]` names, or one of a collection's value it
/// holds `None` of as `null`, `set(key, value: V)` of a `Map<K, V>` (ADR
/// 0884, amended).
pub(in crate::lower) fn takes_nullable(tcx: TyCtxt<'_>, def_id: DefId, i: usize) -> bool {
    let names = nullable_params(tcx, def_id);
    let named = (tcx.fn_arg_idents(def_id).get(i).copied().flatten()).is_some_and(|ident| names.contains(&ident.name));
    let input = tcx.fn_sig(def_id).skip_binder().skip_binder().inputs().get(i).copied();
    named
        || input.is_some_and(
            |input| matches!(input.kind(), ty::Param(p) if nullable_type_params(tcx, def_id).contains(&p.index)),
        )
}

/// Whether binding `def_id`'s `i`th parameter holds, in a tuple, a type
/// parameter its `#[rust_js::nullable(..)]` names: a `Map`'s entries,
/// `new Map([[key, value]])`, each value of which is written `null` for
/// `None`, as `set` writes it (ADR 0884, amended).
pub(in crate::lower) fn takes_nullable_entries(tcx: TyCtxt<'_>, def_id: DefId, i: usize) -> bool {
    let params = nullable_type_params(tcx, def_id);
    if params.is_empty() {
        return false;
    }
    let input = tcx.fn_sig(def_id).skip_binder().skip_binder().inputs().get(i).copied();
    input.is_some_and(|input| holds_in_a_tuple(input, &params))
}

/// Whether `ty` holds one of type parameters `params` in a tuple.
fn holds_in_a_tuple(ty: Ty<'_>, params: &[u32]) -> bool {
    ty.walk().any(|part| {
        part.as_type().is_some_and(|t| {
            matches!(t.kind(), ty::Tuple(items) if items.iter().any(|item| {
                item.walk().any(|part| matches!(part.as_type().map(|t| *t.kind()), Some(ty::Param(p)) if params.contains(&p.index)))
            }))
        })
    })
}

/// Whether binding `def_id` is a keyed read, a `Map`'s `get(key)`, of a
/// value whose `None` its collection holds as
/// `null`, the type parameter its `Option` result holds, of itself or a
/// reference to it, being one `#[rust_js::nullable(..)]` names: JS's read
/// alone tells a key it lacks, `undefined`, from one whose value is
/// `None`, `null` (ADR 0884, amended).
pub(in crate::lower) fn reads_a_nullable_value(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    if !super::reads_a_key(tcx, def_id) {
        return false;
    }
    let output = tcx.fn_sig(def_id).skip_binder().skip_binder().output();
    let ty::Adt(adt, args) = output.kind() else {
        return false;
    };
    let held = match args.type_at(0).kind() {
        ty::Ref(_, held, _) => *held,
        _ => args.type_at(0),
    };
    tcx.is_lang_item(adt.did(), LangItem::Option)
        && matches!(held.kind(), ty::Param(p) if nullable_type_params(tcx, def_id).contains(&p.index))
}

/// Report each value given a binding's parameter that holds, in a tuple,
/// a type parameter its `#[rust_js::nullable(..)]` names, where that's an
/// `Option` of what never looks like `None`: `map::new(Some(&entries))`
/// of a `Map<K, Option<&T>>`, whose entries, made before the call, hold a
/// `None` as `undefined`,
/// where the map holds it as `null`, and its `get` would read `None` of a
/// key whose value is `None` (ADR 0884, amended). `None`, of nothing, is
/// given, and entries written where they're given, `Some(&[(key, None)])`,
/// whose values are each written `null` for `None` there. False if there
/// was one.
pub(in crate::lower) fn reject_nullable_held<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    bodies: &[&crate::lower::Body<'tcx>],
) -> bool {
    use crate::lower::body_queries::strip;
    use rustc_middle::thir::ExprKind;
    let tcx = recognition.tcx;
    let mut valid = true;
    for body in bodies {
        let thir = &body.thir;
        for expr in thir.exprs.iter() {
            let ExprKind::Call { fun, ref args, .. } = expr.kind else {
                continue;
            };
            let ty::FnDef(callee, generic_args) = *thir[strip(thir, fun)].ty.kind() else {
                continue;
            };
            if !super::is_binding(tcx, callee) {
                continue;
            }
            // An `Option` of what never looks like `None`, whose `get` is
            // JS's read alone, which tells a `None` by its `null`.
            let of_options: Vec<u32> = (nullable_type_params(tcx, callee).into_iter())
                .filter(|&p| match generic_args.skip_binder().type_at(p as usize).kind() {
                    ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), LangItem::Option) => {
                        !crate::lower::representation::may_look_like_none(recognition, args.type_at(0))
                    }
                    _ => false,
                })
                .collect();
            if of_options.is_empty() {
                continue;
            }
            let inputs = tcx.fn_sig(callee).skip_binder().skip_binder().inputs();
            for (&arg, input) in args.iter().zip(inputs) {
                // Held in a tuple, an entry's, `(&K, V)`: what the program
                // made, not the collection itself, whose `V`s are its own.
                // Of nothing, or entries written where they're given, each
                // value written `null` for `None` there, `new Map([[key,
                // null]])`.
                if holds_in_a_tuple(*input, &of_options) && !written_in_place(thir, arg) {
                    let name = tcx.item_name(callee);
                    tcx.dcx()
                        .struct_span_err(
                            thir[arg].span,
                            format!(
                                "rust-js: `{name}` of `Option` values made before it is refused: the collection holds \
                                 a `None` as `null`, where what it's given holds it as `undefined`"
                            ),
                        )
                        .with_help("make it of nothing and `set` each value (ADR 0884, amended)")
                        .emit();
                    valid = false;
                }
            }
        }
    }
    valid
}

/// Whether `arg` is `None`, or entries written where they're given, an
/// array of tuples, borrowed, reborrowed, coerced or a `Some`'s.
fn written_in_place(thir: &rustc_middle::thir::Thir<'_>, mut arg: rustc_middle::thir::ExprId) -> bool {
    use rustc_middle::thir::ExprKind;
    loop {
        arg = crate::lower::body_queries::strip(thir, arg);
        match &thir[arg].kind {
            ExprKind::Adt(made) if made.variant_index.as_u32() == 0 && made.fields.is_empty() => return true,
            ExprKind::Adt(made) if made.fields.len() == 1 => arg = made.fields[0].expr,
            ExprKind::Borrow { arg: of, .. } | ExprKind::Deref { arg: of } => arg = *of,
            ExprKind::PointerCoercion { source, .. } => arg = *source,
            ExprKind::Array { fields } => {
                return fields.iter().all(|&field| {
                    matches!(
                        thir[crate::lower::body_queries::strip(thir, field)].kind,
                        ExprKind::Tuple { .. }
                    )
                });
            }
            _ => return false,
        }
    }
}
