//! What a crate does that rust-js refuses, found before any of it is lowered.

use crate::lower::bindings;
use crate::lower::recognition::{StdItem, from_serde_derive, is_from_str, is_std_def, known_derive};
use crate::lower::traits;
use crate::lower::{Body, strip};
use rustc_hir::def::DefKind;
use rustc_hir::find_attr;
use rustc_middle::middle::codegen_fn_attrs::CodegenFnAttrFlags;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{AdtExprBase, ExprId, ExprKind, Thir};
use rustc_middle::ty;
use rustc_middle::ty::TyCtxt;
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
    // What the crate exports by a symbol of its own, `#[no_mangle]` or
    // `#[export_name]`: an `extern` declaration of one is a JS binding
    // naming a global no JS has, and the function is the crate's, by its path.
    let exported: HashSet<Symbol> = tcx
        .hir_crate_items(())
        .definitions()
        .filter(|&id| matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn) && !tcx.is_foreign_item(id))
        .filter_map(|id| {
            let attrs = tcx.codegen_fn_attrs(id);
            match attrs.flags.contains(CodegenFnAttrFlags::NO_MANGLE) {
                true => Some(attrs.symbol_name.unwrap_or_else(|| tcx.item_name(id.to_def_id()))),
                false => attrs.symbol_name,
            }
        })
        .collect();
    for def_id in tcx.hir_crate_items(()).definitions() {
        let what = match tcx.def_kind(def_id) {
            DefKind::Fn
                if tcx.is_foreign_item(def_id)
                    && exported.contains(
                        &tcx.codegen_fn_attrs(def_id)
                            .symbol_name
                            .unwrap_or_else(|| tcx.item_name(def_id.to_def_id())),
                    ) =>
            {
                "an `extern` declaration of this crate's own `#[no_mangle]` function"
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
                let path = tcx.def_path_str(trait_id);
                tcx.dcx().span_err(
                    tcx.def_span(def_id),
                    format!("rust-js does not support user implementations of `{path}` yet"),
                );
                valid = false;
                continue;
            }
            DefKind::Static { .. } if tcx.is_thread_local_static(def_id.to_def_id()) => "`#[thread_local]` statics",
            _ => continue,
        };
        tcx.dcx()
            .span_err(tcx.def_span(def_id), format!("rust-js does not support {what} yet"));
        valid = false;
    }
    valid
}

/// Report each reference rust-js can't make to a static yet (ADR 0096): a
/// `&mut` to a `static mut` or a part of one, and a raw address of any. A
/// shared one is the value it points to, as any is. False if there was one.
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
            let what = match static_of(thir, arg) {
                Some(d) if tcx.is_foreign_item(d) => continue,
                Some(_) if raw => "raw addresses of statics",
                Some(d) if tcx.is_mutable_static(d) => "`&mut` references to a `static mut`",
                _ => continue,
            };
            tcx.dcx()
                .span_err(expr.span, format!("rust-js does not support {what} yet"));
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
pub(super) fn init_of(tcx: TyCtxt<'_>, d: LocalDefId) -> Option<LocalDefId> {
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

/// Report what flattened props (ADR 0204, 0205) can't be: a struct with
/// more than one rest, or a flattened field that isn't a struct of named
/// fields; a flattened field read whole, which JS's props don't have; and
/// a struct with one made anywhere but as JSX's props, or a flattened
/// field's value there, which JSX holds flat. False if there was one.
pub(super) fn reject_flatten_misuse<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> bool {
    let mut valid = true;
    let mut refuse = |span: Span, what: String| {
        tcx.dcx().span_err(span, format!("rust-js does not support {what} yet"));
        valid = false;
    };
    for id in tcx.hir_crate_items(()).definitions() {
        if tcx.def_kind(id) != DefKind::Struct {
            continue;
        }
        let parent = tcx.adt_def(id).non_enum_variant();
        let Some(field) = parent.fields.iter().find(|f| bindings::is_flatten(tcx, f)) else {
            continue;
        };
        let span = tcx.def_span(field.did);
        let own = tcx.type_of(id).instantiate_identity().skip_normalization();
        if (0..parent.fields.len())
            .filter(|&i| bindings::is_rest_field(tcx, own, i))
            .count()
            > 1
        {
            refuse(
                span,
                "props with more than one rest: they have one rest, a `Rest` or a flattened field".into(),
            );
            continue;
        }
        let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
        if !ty
            .ty_adt_def()
            .is_some_and(|adt| adt.is_struct() && adt.non_enum_variant().ctor.is_none())
        {
            refuse(
                span,
                format!("flattening `{ty}`: a flattened field is a struct of named fields"),
            );
        }
    }
    for body in all_bodies {
        let thir = &body.thir;
        // Each struct JSX gives a component as its props, `<*>`'s second,
        // with its base, `{..Default::default()}`, and its flattened
        // fields' values, made there as structs or defaults: JSX takes
        // them apart.
        let mut made = Vec::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let ty::FnDef(def_id, _) = *thir[fun].ty.kind()
                && matches!(bindings::js_form(tcx, def_id), bindings::JsForm::Jsx(tag) if tag == "*")
                && let [_, given] = args[..]
            {
                made.push(strip(thir, given));
            }
        }
        let mut props = HashSet::new();
        // What's a base of them, `..props.html`: a flattened field read whole
        // there is the object its parent is, `{...props}`.
        let mut bases = HashSet::new();
        while let Some(e) = made.pop() {
            props.insert(e);
            if let ExprKind::Adt(ref adt) = thir[e].kind {
                if let AdtExprBase::Base(ref fru) = adt.base {
                    bases.insert(strip(thir, fru.base));
                    made.push(strip(thir, fru.base));
                }
                for field in &adt.fields {
                    if bindings::is_flatten_field(tcx, thir[e].ty, field.name.as_usize()) {
                        made.push(strip(thir, field.expr));
                    }
                }
            }
        }
        // A flattened field read through, `props.html.title`, is its parent's.
        let read_through: HashSet<ExprId> = (thir.exprs.iter())
            .filter_map(|expr| match expr.kind {
                ExprKind::Field { lhs, .. } => Some(strip(thir, lhs)),
                _ => None,
            })
            .collect();
        for (e, expr) in thir.exprs.iter_enumerated() {
            match expr.kind {
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
                ExprKind::Adt(_) | ExprKind::Call { .. }
                    if bindings::has_flatten(tcx, expr.ty) && !props.contains(&e) =>
                {
                    refuse(
                        expr.span,
                        "props with a flattened field made here: they're made only as JSX's, `<Card anchor={..} />`"
                            .into(),
                    );
                }
                _ => {}
            }
        }
    }
    valid
}
