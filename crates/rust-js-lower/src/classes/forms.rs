//! A class's form (ADR 0761, amended, rd682, #2495): an ES class, `class
//! Plugin { constructor(..) { .. } apply(..) { .. } }`, as a bundler drops
//! one nothing uses; or, of one whose prototype the program replaces, which
//! an ES class's can't be, a constructor function and its prototype; and
//! what's refused of it.

use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

use super::{is_class, superclass};

/// The word of `#[rust_js::class(es)]`, a class that's an ES class.
const ES: &str = "es";

/// The word of `#[rust_js::class(es, fields)]`, an ES class that declares
/// its fields (#568).
const FIELDS: &str = "fields";

/// What `#[rust_js::class(..)]` says of `def_id`'s form, the words in its
/// parentheses, `es` of `#[rust_js::class(es)]`; none of `#[rust_js::class]`.
fn class_form(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<String> {
    (tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("class")]))
        .flat_map(|attr| attr.meta_item_list().unwrap_or_default())
        .map(|item| item.ident().map_or_else(String::new, |ident| ident.name.to_string()))
        .collect()
}

/// Is `adt`, a class, an ES class, `class Plugin { constructor(..) { .. }
/// apply(..) { .. } }`: every class (Tu's decision of 2026-11-24, #2495),
/// a declaration a bundler drops where nothing uses it, where a constructor
/// function's prototype assignments are its module's statements, which it
/// keeps; but one whose prototype the program replaces, `js::class::<T>()
/// .set_prototype(..)`, which JS doesn't let an ES class's be, React's
/// `PureComponent` and the Flight client's `ReactPromise`: a constructor
/// function and its prototype (ADR 0761). One that extends a binding's
/// class (rd331), or marked `#[rust_js::class(es)]` (rd682), is one always,
/// its prototype's replacement refused (`es_prototype_refused`).
pub(crate) fn is_es_class(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    is_class(tcx, adt)
        && (class_form(tcx, adt).iter().any(|word| word == ES)
            || superclass(tcx, adt).is_some()
            || !prototype_replaced(tcx, adt))
}

/// Does the program replace `adt`'s prototype, `js::class::<T>()
/// .set_prototype(..)` of it written anywhere in the crate?
fn prototype_replaced(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    let caches = crate::session::caches(tcx);
    let replaced = caches
        .prototype_replaced
        .get_or_init(|| std::rc::Rc::new(replaced_prototypes(tcx)));
    replaced.contains(&adt)
}

/// Each of the crate's classes whose prototype one of its bodies replaces,
/// `js::class::<ReactPromise>().set_prototype(..)`, of the receiver as
/// written: `js::class::<T>()` of the crate's class, as `liveness` finds it.
fn replaced_prototypes(tcx: TyCtxt<'_>) -> rust_js_program::collections::IndexSet<DefId> {
    use rustc_hir::{self as hir, intravisit};
    struct Find<'tcx> {
        tcx: TyCtxt<'tcx>,
        results: &'tcx rustc_middle::ty::TypeckResults<'tcx>,
        found: rust_js_program::collections::IndexSet<DefId>,
    }
    impl<'tcx> intravisit::Visitor<'tcx> for Find<'tcx> {
        type NestedFilter = rustc_middle::hir::nested_filter::OnlyBodies;
        fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
            self.tcx
        }
        fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
            if let hir::ExprKind::MethodCall(_, receiver, _, _) = expr.kind
                && let Some(method) = self.results.type_dependent_def_id(expr.hir_id)
                && crate::bindings::is_binding(self.tcx, method)
                && matches!(crate::bindings::js_form(self.tcx, method), crate::bindings::JsForm::Set(name) if name == "prototype")
                && let hir::ExprKind::Call(callee, _) = receiver.kind
                && let Some((class, args)) = crate::fn_def(self.results.expr_ty(callee))
                && crate::bindings::is_binding(self.tcx, class)
                && matches!(
                    crate::bindings::js_form(self.tcx, class),
                    crate::bindings::JsForm::Class
                )
                && let Some(rustc_middle::ty::Adt(adt, _)) = args.types().next().map(|t| t.kind())
                && adt.did().is_local()
            {
                self.found.insert(adt.did());
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut found = rust_js_program::collections::IndexSet::default();
    for owner in tcx.hir_body_owners() {
        let mut find = Find {
            tcx,
            results: tcx.typeck(owner),
            found: std::mem::take(&mut found),
        };
        intravisit::Visitor::visit_body(&mut find, tcx.hir_body_owned_by(owner));
        found = find.found;
    }
    found
}

/// Does `adt`, a class, declare its fields, `#[rust_js::class(es, fields)]`,
/// `class Plugin { chunkName; constructor(..) { .. } }`, as its author
/// writes it where upstream does (#568, the user's decision of 2026-11-04)?
pub(crate) fn declares_fields(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    is_class(tcx, adt) && matches!(class_form(tcx, adt).as_slice(), [es, fields] if es == ES && fields == FIELDS)
}

/// Each of the crate's ES classes (`is_es_class`).
pub(crate) fn es_classes(tcx: TyCtxt<'_>) -> Vec<DefId> {
    (tcx.hir_crate_items(()).definitions())
        .map(|id| id.to_def_id())
        .filter(|&id| tcx.def_kind(id) == DefKind::Struct && is_es_class(tcx, id))
        .collect()
}

/// Why `adt`, an ES class, has its prototype replaced refused: JS doesn't
/// let an ES class's `prototype` be written, and a load's assignment would
/// throw; a constructor function's is (#448, rd682).
pub(crate) fn es_prototype_refused(tcx: TyCtxt<'_>, adt: DefId) -> String {
    let class = tcx.item_name(adt);
    match superclass(tcx, adt) {
        Some((_, base)) => format!(
            "replacing the prototype of `{class}`, an ES class extending `{}`, whose `prototype` JS doesn't let be written",
            base.name()
        ),
        None => format!(
            "replacing the prototype of `{class}`, an ES class, `#[rust_js::class(es)]`, whose `prototype` JS doesn't let be written: unmarked, `#[rust_js::class]`, one whose prototype the program replaces is a constructor function, whose is (#2495)"
        ),
    }
}

/// What's refused of the form `class`'s struct says, which extends a
/// binding's class where `extends`: a word that's no form, `es` of one
/// that's an ES class already, by its impl (one way), and `fields` of one
/// that isn't `es`, which only an ES class declares.
pub(super) fn form_misuse(tcx: TyCtxt<'_>, class: DefId, extends: bool) -> Option<&'static str> {
    match class_form(tcx, class).as_slice() {
        [] => None,
        [word, ..] if word == ES && extends => {
            Some("that extends a binding's class is an ES class already: write it `#[rust_js::class]`")
        }
        [word] if word == ES => None,
        [es, fields] if es == ES && fields == FIELDS => None,
        words if words.iter().any(|word| word == FIELDS) => Some(
            "declares its fields as an ES class, `#[rust_js::class(es, fields)]`: a constructor function has no field declarations",
        ),
        _ => Some(
            "is a class, `#[rust_js::class]`, an ES class but one whose prototype the program replaces, or one that declares its fields, `#[rust_js::class(es, fields)]`",
        ),
    }
}

/// What's refused of a class's impl, `imp`, marked with a form: its
/// struct's attribute says it, once.
pub(super) fn impl_form_misuse(tcx: TyCtxt<'_>, imp: DefId) -> Option<&'static str> {
    (!class_form(tcx, imp).is_empty()).then_some(
        "impl is marked `#[rust_js::class]` alone: its struct's attribute says its form, `#[rust_js::class(es)]`",
    )
}
