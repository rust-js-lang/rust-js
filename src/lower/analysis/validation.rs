//! What a crate does that rust-js refuses, found before any of it is lowered.

use crate::lower::bindings;
use crate::lower::recognition::{StdItem, from_serde_derive, is_from_str, is_std_def, known_derive};
use crate::lower::refusal::{Refused, refuse};
use crate::lower::representation::marks_js_object;
use crate::lower::traits;
use crate::lower::{Body, strip};
use rustc_hir::def::DefKind;
use rustc_hir::find_attr;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind, PatKind, Thir};
use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::{DefId, LocalDefId};
use rustc_span::{Span, Symbol};
use std::collections::HashSet;

/// Report each item rust-js can't compile yet. False if there was one.
pub(super) fn reject_unsupported(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    markers: &[(LocalDefId, Symbol)],
) -> bool {
    let mut valid = true;
    for def_id in tcx.hir_crate_items(()).definitions() {
        let what = match tcx.def_kind(def_id) {
            // A function in an `extern "Rust"` block, where a JS function
            // was bound: a binding of one is an ordinary function, the one
            // form (ADR 0373). One of C's, which JS has none of, is refused
            // where it's called.
            DefKind::Fn
                if tcx.is_foreign_item(def_id)
                    && tcx.fn_sig(def_id).skip_binder().abi() == rustc_abi::ExternAbi::Rust =>
            {
                let refused = super::super::bindings::extern_fn_refused(tcx, def_id.to_def_id());
                tcx.dcx().span_err(tcx.def_span(def_id), refused);
                valid = false;
                continue;
            }
            // An `extern` type: a JS type is one form, a struct of a
            // `JsObject` (ADR 0111), stable Rust.
            DefKind::ForeignTy => {
                let name = tcx.item_name(def_id.to_def_id());
                tcx.dcx().span_err(
                    tcx.def_span(def_id),
                    format!(
                        "rust-js: a JS type is declared as a struct of a `JsObject`, `pub struct {name}(PhantomData<js::JsObject>);`, not an `extern` type (ADR 0373)"
                    ),
                );
                valid = false;
                continue;
            }
            // `#[eii] static HELLO: u64;`, which the linker makes another
            // item: rust-js has none to link it to, and JS would read a name
            // nothing defines (ADR 0109).
            _ if find_attr!(tcx, def_id, EiiImpl(..) | EiiDeclaration(..) | RustcEiiForeignItem) => {
                "externally implementable items"
            }
            _ if markers.iter().any(|&(marker, _)| marker == def_id) => continue,
            _ if from_serde_derive(tcx, def_id) => continue,
            // std's storage for a thread-local: JS needs none.
            _ if in_thread_local(tcx, def_id).is_some() => continue,
            // Methods, trait impls' included (ADRs 0047, 0049). Derives like
            // `#[derive(Clone)]` write impls that are never called.
            DefKind::AssocFn => continue,
            // A `type const`, of `min_generic_const_args`, has no body to type-check.
            DefKind::AssocConst { is_type_const: true } => "type constants",
            // A type's own `const`, as `Vec2::ZERO`: its value where it's used,
            // as rustc computed it (ADR 0031). A trait's too, and in generic code
            // its impl's dictionary's (ADR 0106). Not one with parameters of its own.
            DefKind::AssocConst { .. } if tcx.generics_of(def_id).own_params.is_empty() => continue,
            DefKind::AssocConst { .. } => "generic constants",
            // An `Iterator`'s `Item` (ADR 0055), an operator's `Output` (ADR
            // 0064) and a `TryFrom`'s `Error`: rustc works out what they are.
            DefKind::AssocTy
                if tcx.trait_impl_of_assoc(def_id.to_def_id()).is_some_and(|imp| {
                    let tr = tcx
                        .impl_trait_ref(imp)
                        .instantiate_identity()
                        .skip_normalization()
                        .def_id;
                    is_std_def(tcx, tr, StdItem::Iterator)
                        || is_std_def(tcx, tr, StdItem::TryFrom)
                        || is_from_str(tcx, tr)
                        || is_std_def(tcx, tr, StdItem::IntoIterator)
                        || traits::is_operator(tcx, tr)
                }) =>
            {
                continue;
            }
            // A trait's own, a type only a caller knows in generic code, as a
            // type parameter is (ADR 0106); a generic one too, of lifetimes,
            // which JS hasn't, or of no bound a dictionary has (ADR 0146).
            DefKind::AssocTy if traits::gat_supported(tcx, def_id.to_def_id()) => continue,
            DefKind::AssocTy => "generic associated types with bounds",
            // Named, as which one stops a crate is what's worth knowing.
            DefKind::Impl { of_trait: true } if !known_derive(tcx, def_id.to_def_id()) => {
                let trait_id = tcx
                    .impl_trait_ref(def_id)
                    .instantiate_identity()
                    .skip_normalization()
                    .def_id;
                if traits::implementable(tcx, foreign, trait_id) {
                    continue;
                }
                let what = format!("user implementations of `{}`", tcx.def_path_str(trait_id));
                refuse(tcx, tcx.def_span(def_id), Refused::Feature(&what));
                valid = false;
                continue;
            }
            DefKind::Static { .. } if tcx.is_thread_local_static(def_id.to_def_id()) => "`#[thread_local]` statics",
            _ => continue,
        };
        refuse(tcx, tcx.def_span(def_id), Refused::Feature(what));
        valid = false;
    }
    valid
}

/// Report each reference rust-js can't make to a static yet (ADR 0096): a
/// `&mut` to a `static mut` or a part of one, and a raw address of an
/// immutable one. A shared one is the value it points to, as any is; a raw
/// pointer to a `static mut`, `&raw mut COUNT`, a handle on its place (ADRs
/// 0096, amended, 0407), which its module keeps its box for where it's
/// given on (`std_types/cell.rs::plain_statics`). False if there was one.
pub(super) fn reject_static_references(tcx: TyCtxt<'_>, all_bodies: &[&Body<'_>]) -> bool {
    let mut valid = true;
    for body in all_bodies {
        let thir = &body.thir;
        for expr in thir.exprs.iter() {
            let (arg, raw) = match expr.kind {
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } => (arg, false),
                ExprKind::RawBorrow { arg, .. } => (arg, true),
                _ => continue,
            };
            let refused = match static_of(thir, arg) {
                Some(d) if tcx.is_foreign_item(d) => continue,
                Some(d) if raw && !tcx.is_mutable_static(d) => Refused::Memory("raw addresses of statics"),
                Some(_) if raw => continue,
                Some(d) if tcx.is_mutable_static(d) => Refused::Feature("`&mut` references to a `static mut`"),
                _ => continue,
            };
            refuse(tcx, expr.span, refused);
            valid = false;
        }
    }
    valid
}

/// The static that place `e` is, or is a part of.
pub(super) fn static_of(thir: &Thir<'_>, e: ExprId) -> Option<DefId> {
    match thir[strip(thir, e)].kind {
        ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } => static_of(thir, lhs),
        ExprKind::Deref { arg } => match thir[strip(thir, arg)].kind {
            ExprKind::StaticRef { def_id, .. } => Some(def_id),
            _ => None,
        },
        _ => None,
    }
}

/// `thread_local!` (ADR 0037) is a `const NAME: LocalKey<T>` whose block holds
/// `fn __rust_std_internal_init_fn() -> T { init }`, then std's storage for
/// it. In JS, it's a variable of its module, made from `init`.
pub(in crate::lower) fn is_thread_local(tcx: TyCtxt<'_>, d: LocalDefId) -> bool {
    matches!(tcx.def_kind(d), DefKind::Const { .. })
        && matches!(tcx.type_of(d).instantiate_identity().skip_normalization().kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::LocalKey))
}

/// The thread-local whose block `d` is in, if any.
/// The thread-local whose `init` `d` is: one inside it, with no function
/// between, as a function the `init` makes is its own (ADR 0296).
pub(in crate::lower) fn init_of(tcx: TyCtxt<'_>, d: LocalDefId) -> Option<LocalDefId> {
    let mut parent = tcx.opt_local_parent(d);
    while let Some(p) = parent {
        if is_thread_local(tcx, p) {
            return Some(p);
        }
        if matches!(tcx.def_kind(p), DefKind::Fn | DefKind::AssocFn | DefKind::Closure) {
            return None;
        }
        parent = tcx.opt_local_parent(p);
    }
    None
}

pub(super) fn in_thread_local(tcx: TyCtxt<'_>, d: LocalDefId) -> Option<LocalDefId> {
    let mut parent = tcx.opt_local_parent(d);
    while let Some(p) = parent {
        if is_thread_local(tcx, p) {
            return Some(p);
        }
        parent = tcx.opt_local_parent(p);
    }
    None
}

/// Report what flattened props (ADR 0204, 0205) can't be: a flattened field
/// that isn't a struct of named fields; a flattened field read whole,
/// which JS's props don't have; one of a struct of more than one rest read
/// through, or such a struct taken apart, whose rests JS's object mixes;
/// and a struct with one made anywhere but as JSX's props, or a flattened
/// field's value there, which JSX holds flat. False if there was one.
pub(super) fn reject_flatten_misuse<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> bool {
    let mut valid = true;
    let mut refuse = |span: Span, what: String| {
        refuse(tcx, span, Refused::Feature(&what));
        valid = false;
    };
    // The structs of more than one rest: made, `{ ...template, ...files }`,
    // but neither taken apart nor read through, as JS's object has their
    // keys mixed.
    let mut mixed = HashSet::new();
    for id in tcx.hir_crate_items(()).definitions() {
        // An enum's variant's, which nothing makes flat: it'd be nested.
        if tcx.def_kind(id) == DefKind::Enum {
            let variants = tcx.adt_def(id).variants();
            for field in variants
                .iter()
                .flat_map(|v| v.fields.iter())
                .filter(|f| bindings::is_flatten(tcx, f))
            {
                refuse(tcx.def_span(field.did), "flattening an enum variant's field".into());
            }
            continue;
        }
        if tcx.def_kind(id) != DefKind::Struct {
            continue;
        }
        let parent = tcx.adt_def(id).non_enum_variant();
        let own = tcx.type_of(id).instantiate_identity().skip_normalization();
        if (0..parent.fields.len())
            .filter(|&i| bindings::is_rest_field(tcx, own, i))
            .count()
            > 1
        {
            mixed.insert(id.to_def_id());
        }
        for field in parent.fields.iter().filter(|f| bindings::is_flatten(tcx, f)) {
            let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
            // A struct, or a reference to one, which is the object it is; or a
            // JS object, a `Dict` whose keys spread.
            let object = match ty.peel_refs().kind() {
                ty::Adt(adt, args) => {
                    adt.is_struct() && adt.non_enum_variant().ctor.is_none() || marks_js_object(tcx, *adt, args)
                }
                _ => false,
            };
            if !object {
                refuse(
                    tcx.def_span(field.did),
                    format!("flattening `{ty}`: a flattened field is a struct of named fields or a JS object"),
                );
            }
        }
    }
    let is_mixed = |ty: Ty<'tcx>| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if mixed.contains(&adt.did()));
    for body in all_bodies {
        let thir = &body.thir;
        // Taken apart where they're given, which JS's one rest is.
        for param in thir.params.iter() {
            if let Some(pat) = &param.pat
                && matches!(pat.kind, PatKind::Leaf { .. })
                && is_mixed(pat.ty)
            {
                refuse(
                    pat.span,
                    "props with more than one rest taken apart: they have one rest, a `Rest` or a flattened field"
                        .into(),
                );
            }
        }
        let bases = &body.facts.jsx_bases;
        // A flattened field read through, `props.html.title`, is its parent's,
        // a reference's too, `(*report.item).line`.
        let read_through: HashSet<ExprId> = (thir.exprs.iter())
            .filter_map(|expr| match expr.kind {
                ExprKind::Field { lhs, .. } => {
                    let mut lhs = strip(thir, lhs);
                    while let ExprKind::Deref { arg } = thir[lhs].kind {
                        lhs = strip(thir, arg);
                    }
                    Some(lhs)
                }
                _ => None,
            })
            .collect();
        for (e, expr) in thir.exprs.iter_enumerated() {
            match expr.kind {
                ExprKind::Field { lhs, name, .. }
                    if is_mixed(thir[lhs].ty) && bindings::is_flatten_field(tcx, thir[lhs].ty, name.as_usize()) =>
                {
                    refuse(
                        expr.span,
                        "reading props of more than one rest through one of them: JS's object has their keys mixed"
                            .into(),
                    );
                }
                ExprKind::Field { lhs, name, .. }
                    if bindings::is_flatten_field(tcx, thir[lhs].ty, name.as_usize())
                        && !read_through.contains(&e)
                        && !bases.contains(&e) =>
                {
                    refuse(
                        expr.span,
                        "reading flattened props whole: take them apart where they're given, `Props { a, anchor }: Props`, or read through them, `anchor.html.title`".into(),
                    );
                }
                _ => {}
            }
        }
    }
    valid
}

/// Report each `#[rust_js::rest]` that isn't JS's rest parameter (ADR
/// 0664): one of a closure, given what its type says, of a trait's method,
/// which each impl's call gives alike, but a default of a trait of classes,
/// each class's prototype's (rd283), or of a component, which React gives
/// its props; one not the last parameter, not a shared slice's, or not a
/// name. Each says what to write. False if there was one.
pub(super) fn reject_rest_misuse(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    for owner in tcx.hir_body_owners() {
        let def_id = owner.to_def_id();
        let count = tcx.hir_body_owned_by(owner).params.len();
        for (index, param) in bindings::marked_rest(tcx, owner) {
            let ty = tcx.typeck(owner).node_type(param.hir_id);
            let name = match param.pat.kind {
                rustc_hir::PatKind::Binding(_, _, ident, None) => Some(ident),
                _ => None,
            };
            let refused = if tcx.is_closure_like(def_id) {
                "a function's last parameter, not a closure's: a closure takes rest arguments as a value, `args: &dyn js::Rest<T>`, which its type says".to_string()
            } else if tcx.trait_impl_of_assoc(def_id).is_some()
                || tcx
                    .trait_of_assoc(def_id)
                    .is_some_and(|t| !crate::lower::classes::is_class_trait(tcx, t))
            {
                "a function's last parameter, not a trait method's, which each impl's call gives alike".to_string()
            } else if bindings::is_component(tcx, def_id) {
                "a function's last parameter, not a component's, which React gives its props".to_string()
            } else if index + 1 != count {
                "a function's last parameter, as JS's rest parameter is".to_string()
            } else if !matches!(ty.kind(), ty::Ref(_, slice, rustc_hir::Mutability::Not) if slice.is_slice()) {
                let name = name.map_or_else(|| "args".to_string(), |ident| ident.to_string());
                // As it's written, `Vec<f64>`.
                let written = tcx.sess.source_map().span_to_snippet(param.ty_span);
                format!(
                    "a slice's, `{name}: &[T]`, not a `{}`'s",
                    written.unwrap_or_else(|_| ty.to_string())
                )
            } else if name.is_none() {
                "a parameter's name, `args: &[T]`, not a pattern's: JS's rest parameter is an array".to_string()
            } else {
                continue;
            };
            tcx.dcx()
                .span_err(param.span, format!("rust-js: `#[rust_js::rest]` is {refused}"));
            valid = false;
        }
    }
    valid
}

/// Report each parameter's and `let`'s `#[rust_js::name]` that names
/// nothing it can (ADR 0855, amended): one of a pattern, or of `self`,
/// which has a name of its own; not a string; not a name a JS variable may
/// have; or the name it has anyway, a second way to write it (ADR 0573).
/// Each says what to write. False if there was one.
pub(super) fn reject_parameter_name_misuse(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    for owner in tcx.hir_body_owners() {
        for (local, attr) in bindings::named_lets(tcx, owner) {
            let rustc_hir::PatKind::Binding(_, _, ident, None) = local.pat.kind else {
                tcx.dcx().span_err(
                    attr.span(),
                    "rust-js: `#[rust_js::name]` names a `let` that binds a name, `let value = ..`, not a pattern",
                );
                valid = false;
                continue;
            };
            let Some(given) = attr.value_str() else {
                tcx.dcx().span_err(
                    attr.span(),
                    "rust-js: write a `let`'s JS name as a string, `#[cfg_attr(rust_js, rust_js::name = \"$$typeof\")]`",
                );
                valid = false;
                continue;
            };
            let default = crate::lower::camel_case(ident.name.as_str());
            valid &= names_a_variable(tcx, attr, ident, given, &default, "a `let`'s");
        }
        for (param, attr) in bindings::named_parameters(tcx, owner) {
            let ident = match param.pat.kind {
                rustc_hir::PatKind::Binding(_, _, ident, None) if ident.name != rustc_span::symbol::kw::SelfLower => {
                    ident
                }
                _ => {
                    tcx.dcx().span_err(
                        attr.span(),
                        "rust-js: `#[rust_js::name]` names a parameter that's a name, `context: T`, not a pattern or `self`",
                    );
                    valid = false;
                    continue;
                }
            };
            let Some(given) = attr.value_str() else {
                tcx.dcx().span_err(
                    attr.span(),
                    "rust-js: write a parameter's JS name as a string, `#[cfg_attr(rust_js, rust_js::name = \"Context\")]`",
                );
                valid = false;
                continue;
            };
            let default = crate::lower::default_parameter_name(tcx, owner.to_def_id(), ident.name.as_str());
            valid &= names_a_variable(tcx, attr, ident, given, &default, "a parameter's");
        }
    }
    valid
}

/// Whether a binding's `#[rust_js::name = "given"]` is a name a JS variable
/// may have, and not the `default` it has anyway (ADR 0855, amended):
/// reported where it isn't, `what` saying whose it is.
fn names_a_variable(
    tcx: TyCtxt<'_>,
    attr: &rustc_hir::Attribute,
    ident: rustc_span::Ident,
    given: rustc_span::Symbol,
    default: &str,
    what: &str,
) -> bool {
    if !crate::names::is_identifier(given.as_str()) || crate::names::kept_word(given.as_str()) {
        tcx.dcx().span_err(
            attr.span(),
            format!("rust-js: {what} JS name is a name JS lets a variable have, not `{given}`"),
        );
        false
    } else if given.as_str() == default {
        tcx.dcx().span_err(
            attr.span(),
            format!("rust-js: `{ident}` is `{given}` in JS without `#[rust_js::name = \"{given}\"]`: remove it"),
        );
        false
    } else {
        true
    }
}
