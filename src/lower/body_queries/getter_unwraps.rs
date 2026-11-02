//! A binding getter's function unwrapped where it's called (rd519, the
//! user's decision): `console::create_task().unwrap()(name)` and
//! `function::bind(console::create_task().unwrap(), ..)`, whose `unwrap()`
//! is the getter's read, `console.createTask(name)` and
//! `console.createTask.bind(console, ..)`, as upstream writes them.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use std::collections::HashSet;

use super::strip;
use crate::lower::bindings::{self, JsForm};

/// Each `unwrap()` of what a binding's getter gives, an
/// `Option` of a function or an object, read only as the callee of the
/// call it's made for, or as the `this` of a binding's method, the one
/// place its value goes: its `fn_span`. Its value is the getter's read
/// itself; of a `None`, JS's call or method read throws a `TypeError`
/// where Rust's `unwrap()` panics (ADR 0262's exception, ADR 1066).
pub(super) fn getter_unwraps<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<Span> {
    let mut found = HashSet::new();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let ty::FnDef(def_id, _) = *thir[fun].ty.kind() else {
            continue;
        };
        // A call of what it gives, through `Fn`, `FnMut` or `FnOnce`, or a
        // binding's method of it, `this.bind(..)`.
        let called = (tcx.trait_of_assoc(def_id)).is_some_and(|t| {
            [LangItem::Fn, LangItem::FnMut, LangItem::FnOnce]
                .iter()
                .any(|&item| tcx.is_lang_item(t, item))
        });
        let receiver = bindings::is_binding(tcx, def_id) && bindings::is_method(tcx, def_id);
        if !called && !receiver {
            continue;
        }
        let Some(&first) = args.first() else { continue };
        if let Some(span) = unwrapped_getter(tcx, thir, first) {
            found.insert(span);
        }
    }
    found
}

/// The `fn_span` of the `unwrap()` `e` is, through borrows and derefs, of
/// a binding getter's call.
fn unwrapped_getter<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, mut e: ExprId) -> Option<Span> {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
            _ => break,
        }
    }
    let ExprKind::Call {
        fun, ref args, fn_span, ..
    } = thir[e].kind
    else {
        return None;
    };
    let ty::FnDef(unwrap, _) = *thir[fun].ty.kind() else {
        return None;
    };
    let of_option = (tcx.inherent_impl_of_assoc(unwrap)).is_some_and(|imp| {
        matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
            ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option))
    });
    if !of_option || tcx.item_name(unwrap).as_str() != "unwrap" {
        return None;
    }
    let [option] = args[..] else { return None };
    let ExprKind::Call { fun: getter, .. } = thir[strip(thir, option)].kind else {
        return None;
    };
    let ty::FnDef(getter, _) = *thir[getter].ty.kind() else {
        return None;
    };
    (bindings::is_binding(tcx, getter) && matches!(bindings::js_form(tcx, getter), JsForm::Get(_))).then_some(fn_span)
}
