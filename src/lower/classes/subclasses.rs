//! A class that extends a binding's class (rd331, ADR 1041): which a
//! binding's trait is, `react#Component<P, S>`, what a class extends, an ES
//! class, and what's refused of its impl of that class.

use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

use super::{class_of, class_of_impl, home, is_class};
use crate::lower::recognition::{StdItem, is_std_def};

/// The JS class `trait_id` is, a binding's trait of methods whose
/// `#[rust_js::types]` names a module's export, react's `Component`,
/// `react#Component<P, S>`: the export a class that implements it extends,
/// `("react", "Component")` (rd331). `None` of any other trait, a marker
/// or a union, react's `ReactNode`.
pub(crate) fn base_class(tcx: TyCtxt<'_>, trait_id: DefId) -> Option<(String, String)> {
    if trait_id.is_local() || tcx.def_kind(trait_id) != DefKind::Trait {
        return None;
    }
    let items = tcx.associated_item_def_ids(trait_id);
    if !items.iter().any(|&item| tcx.def_kind(item) == DefKind::AssocFn) {
        return None;
    }
    match crate::lower::declarations::written_types(tcx, trait_id)? {
        crate::lower::declarations::Written::Reference {
            from: Some(from), name, ..
        } => Some((from, name)),
        _ => None,
    }
}

/// The class `imp` is an impl of a binding's class of, `impl
/// Component<SurfaceProps> for Surface` (rd331): each method it writes is
/// the class's, which extends that class, `class Surface extends Component
/// { componentDidMount() { .. } }`.
pub(crate) fn subclass_impl_class(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    let DefKind::Impl { of_trait: true } = tcx.def_kind(imp) else {
        return None;
    };
    let class = class_of_impl(tcx, imp)?;
    base_class(tcx, tcx.impl_trait_id(imp)).is_some().then_some(class)
}

/// The impls by which `adt`, the crate's class, extends a binding's class
/// (rd331): one, of a class that extends one; more are refused.
pub(super) fn superclass_impls(tcx: TyCtxt<'_>, adt: DefId) -> Vec<DefId> {
    if !adt.is_local() || !is_class(tcx, adt) {
        return Vec::new();
    }
    (tcx.all_local_trait_impls(()).iter())
        .filter(|(trait_id, _)| base_class(tcx, **trait_id).is_some())
        .flat_map(|(_, impls)| impls.iter().map(|imp| imp.to_def_id()))
        .filter(|&imp| class_of_impl(tcx, imp) == Some(adt))
        .collect()
}

/// What `adt`, a class, extends: its impl of a binding's class and that
/// class's export, `("react", "Component")` of `impl Component<P> for
/// Surface`, an ES class's `extends` (rd331); `None` of a class that
/// extends nothing, a constructor function (ADR 0761), and of what's no
/// class.
pub(crate) fn superclass(tcx: TyCtxt<'_>, adt: DefId) -> Option<(DefId, (String, String))> {
    let imp = *superclass_impls(tcx, adt).first()?;
    Some((imp, base_class(tcx, tcx.impl_trait_id(imp))?))
}

/// A function of class `adt` its reference names, `Surface` of
/// `Surface.prototype.render`'s: its constructor, or, of one React
/// constructs, which has none, a method of its impl of the class it
/// extends (rd331i); `None` of what's no class.
pub(crate) fn member_naming(tcx: TyCtxt<'_>, adt: DefId) -> Option<DefId> {
    super::constructor_of(tcx, adt).or_else(|| {
        let (imp, _) = superclass(tcx, adt)?;
        tcx.associated_item_def_ids(imp).first().copied()
    })
}

/// Is `field` one a value is of before it's set, `undefined`: an `Option`,
/// or a `Cell` or a `RefCell` of one, whose `None` it is (rd331); or an
/// optional `OnceCell`, absent until it's set (ADR 0280, amended; rd331l).
pub(super) fn starts_undefined(tcx: TyCtxt<'_>, field: &ty::FieldDef) -> bool {
    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
    let is_option = |ty: ty::Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option));
    match ty.kind() {
        _ if is_option(ty) => true,
        ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::OnceCell) => {
            crate::lower::bindings::is_optional(tcx, field)
        }
        ty::Adt(adt, args) => {
            (is_std_def(tcx, adt.did(), StdItem::Cell) || is_std_def(tcx, adt.did(), StdItem::RefCell))
                && is_option(args.type_at(0))
        }
        _ => false,
    }
}

/// What `imp`, `class`'s impl of the binding's class it extends, is
/// refused of, where: written in another module than its struct's, where
/// `jsx!` renders it, refused at the struct (ADR 1041, amended), or than its
/// constructor's; a method of the binding's, `props`, written; a method
/// named as a field (rd331).
pub(super) fn impl_misuse(tcx: TyCtxt<'_>, imp: DefId, class: DefId) -> Vec<(rustc_span::Span, String)> {
    let mut out = Vec::new();
    let module = tcx.parent_module_from_def_id(imp.expect_local());
    if tcx.parent_module_from_def_id(class.expect_local()) != module {
        out.push((
            tcx.def_span(class),
            format!(
                "that extends a binding's class implements it beside its struct, where `jsx!` renders it: move `impl {}<..> for {}` into the module its struct is in",
                tcx.item_name(tcx.impl_trait_id(imp)),
                tcx.item_name(class)
            ),
        ));
    }
    if home(tcx, class) != Some(module) {
        out.push((
            tcx.def_span(imp),
            "implements the class it extends in the module its constructor is in, where its class is".to_owned(),
        ));
    }
    let fields: Vec<String> = (tcx.adt_def(class).non_enum_variant().fields.iter())
        .filter_map(|field| crate::lower::bindings::field_key(tcx, field).name().map(str::to_owned))
        .collect();
    for &own in tcx.associated_item_def_ids(imp) {
        let Some(item) = tcx.trait_item_of(own) else {
            continue;
        };
        let named = crate::lower::bindings::fn_name(tcx, item);
        if crate::lower::bindings::is_binding(tcx, item) {
            out.push((
                tcx.def_span(own),
                format!(
                    "keeps what the class it extends has, `{named}`, which its impl doesn't write: React's `this.{named}`"
                ),
            ));
        } else if fields.contains(&named) {
            out.push((
                tcx.def_span(own),
                format!(
                    "has no method named as its field, `{named}`, which `this.{named}` would hide: name one otherwise"
                ),
            ));
        }
    }
    out
}

/// The JS class `member`'s class is, made where its first member is
/// placed: what it extends, a binding's, by the name its module imports it
/// as, `imports`', `Component` of react's (rd331); `None` of a class that
/// extends nothing, a constructor function.
pub(crate) fn js_class(
    tcx: TyCtxt<'_>,
    member: DefId,
    imports: &std::collections::HashMap<(String, String), String>,
) -> crate::js::Class {
    crate::js::Class {
        extends: (class_of(tcx, member).and_then(|adt| superclass(tcx, adt)))
            .map(|(_, export)| imports[&export].clone()),
        ..Default::default()
    }
}
