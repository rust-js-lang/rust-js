//! A class that extends JS's `Error` (#1808, ADR 1158): a `#[rust_js::class]`
//! struct that implements `js::ErrorClass`, an ES class, `class VGPUError
//! extends Error`, as ADR 1041's binding's class makes one. What `Error`'s
//! constructor sets is its struct's fields of those names, given it
//! through `super`: `message` its first argument and `cause` its options',
//! `super(data.message, { cause: data.cause })`, so each is the own
//! property JS makes, which no `Object.keys` lists, where `this.message =`
//! would make one it does. Every other field is set after, in Rust's order.

use rust_js_program::js::{self, Expr, ExprKind, Prop, Stmt, StmtKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::{constructor_of, superclass_impls};
use crate::recognition::is_js_error_class;

/// The fields `Error`'s constructor sets, by their JS names.
const MESSAGE: &str = "message";
const CAUSE: &str = "cause";

/// Whether `adt`'s class extends JS's `Error`, implementing `js::ErrorClass`.
pub(crate) fn extends_error(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    (superclass_impls(tcx, adt).first()).is_some_and(|&imp| is_js_error_class(tcx, tcx.impl_trait_id(imp)))
}

/// What's refused of `adt`, a class that extends `Error`, each where and
/// what to write: no constructor, as `Error` makes no instance of a
/// class of its own, and a `message` that isn't text, which JS's own
/// `Error` makes a string of.
pub(super) fn error_class_misuse(tcx: TyCtxt<'_>, adt: DefId) -> Vec<(Span, String)> {
    let mut out = Vec::new();
    if constructor_of(tcx, adt).is_none() {
        out.push((
            tcx.def_span(adt),
            "that extends `Error` writes its constructor, `fn new(..) -> Self` in its `#[rust_js::class]` \
             impl, which JS's `new` calls: `Error` constructs no class of its own (#1808)"
                .to_owned(),
        ));
    }
    for field in tcx.adt_def(adt).non_enum_variant().fields.iter() {
        if crate::bindings::field_key(tcx, field).name() != Some(MESSAGE) {
            continue;
        }
        let ty = tcx
            .type_of(field.did)
            .instantiate_identity()
            .skip_normalization()
            .peel_refs();
        let text = ty.is_str()
            || matches!(ty.kind(), ty::Adt(def, _) if tcx.is_lang_item(def.did(), rustc_hir::attrs::lang_items::LangItem::String));
        if !text {
            out.push((
                tcx.def_span(field.did),
                format!(
                    "that extends `Error` has a `message` of text, `message: String` or `&str`, which \
                     `super(message)` gives `Error`'s constructor: `{ty}` isn't; name the field otherwise (#1808)"
                ),
            ));
        }
    }
    out
}

/// Whether `adt`, a class that extends `Error`, has a field JS names `name`.
fn has_field(tcx: TyCtxt<'_>, adt: DefId, name: &str) -> bool {
    (tcx.adt_def(adt).non_enum_variant().fields.iter())
        .any(|field| crate::bindings::field_key(tcx, field).name() == Some(name))
}

/// Of `body`, the constructor of `adt`, a class that extends `Error`, its
/// `this.message = ..` and `this.cause = ..` made `super(message, { cause
/// })`, its first statement of `this`, at `at`. A field set before them
/// whose value does anything is held first, `const code = ..;`, then set
/// after `super`, in Rust's order. A `cause` field set nowhere, `None`,
/// is `{ cause: undefined }`, there as Rust's field is; of no `cause`
/// field, `super(message)`, as `new Error(message)` has none.
pub(super) fn super_of_fields(
    tcx: TyCtxt<'_>,
    adt: DefId,
    body: &mut Vec<Stmt>,
    at: js::Span,
) -> Result<(), &'static str> {
    let set_of = |stmt: &Stmt, name: &str| {
        matches!(&stmt.kind, StmtKind::Assign(Expr { kind: ExprKind::Member(object, key), .. }, _)
            if key == name && matches!(object.kind, ExprKind::This))
    };
    let (mut message, mut cause) = (None, None);
    let mut before = Vec::new();
    let mut after = Vec::new();
    // The statements before the first that sets `this`'s field: what the
    // body does first, which reads no `this`.
    let first = body
        .iter()
        .position(|stmt| set_of(stmt, MESSAGE) || set_of(stmt, CAUSE) || sets_this(stmt));
    let Some(first) = first else {
        return Err("constructor of a class that extends `Error` that sets no field, which `super(message)` is given");
    };
    let mut rest = body.split_off(first);
    let last_given = rest
        .iter()
        .rposition(|stmt| set_of(stmt, MESSAGE) || set_of(stmt, CAUSE));
    let tail = match last_given {
        Some(last) => rest.split_off(last + 1),
        None => std::mem::take(&mut rest),
    };
    for stmt in rest {
        let span = stmt.span;
        let StmtKind::Assign(target, value) = stmt.kind else {
            return Err(
                "constructor of a class that extends `Error` that does more than set its fields before its `message`, which `super(message)` is given first",
            );
        };
        match &target.kind {
            ExprKind::Member(_, key) if key == MESSAGE => message = Some(value),
            ExprKind::Member(_, key) if key == CAUSE => cause = Some(value),
            ExprKind::Member(_, key) if value.has_effects() => {
                // Named after the field, `code`, else `code1` .. `code7`,
                // else given apart, `code$1`, as preparation spells it
                // apart from the body's names (`Name::suggested`, #2048).
                let preferred = std::iter::once(key.clone())
                    .chain((1..8).map(|n| format!("{key}{n}")))
                    .collect();
                let held = js::Name::declared(key.clone()).suggested(preferred, key);
                before.push(StmtKind::Const(held.clone(), value).at(span));
                after.push(StmtKind::Assign(target, Expr::var(&held)).at(span));
            }
            _ => after.push(StmtKind::Assign(target, value).at(span)),
        }
    }
    let mut given = vec![message.unwrap_or_else(Expr::js_undefined)];
    if has_field(tcx, adt, CAUSE) {
        // Kept of `undefined` too: `Error` makes the property where the key is.
        let cause = cause.unwrap_or_else(Expr::js_undefined);
        let mut options = Expr::object(vec![Prop::Field(CAUSE.to_owned(), cause)]);
        if let ExprKind::Object(_, layout) = &mut options.kind {
            layout.kept.push(CAUSE.to_owned());
        }
        given.push(options);
    } else if !has_field(tcx, adt, MESSAGE) {
        given.clear();
    }
    body.extend(before);
    body.push(StmtKind::Expr(Expr::call(Expr::super_(), given)).at(at));
    body.extend(after);
    body.extend(tail);
    Ok(())
}

/// Whether `stmt` sets a field of `this`, `this.code = ..`.
fn sets_this(stmt: &Stmt) -> bool {
    matches!(&stmt.kind, StmtKind::Assign(Expr { kind: ExprKind::Member(object, _), .. }, _)
        if matches!(object.kind, ExprKind::This))
}
