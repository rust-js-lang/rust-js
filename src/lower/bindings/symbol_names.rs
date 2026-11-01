//! A field named by a symbol, `#[rust_js::name(RESPONSE_SYMBOL)]`: the key
//! `field_key` gives it, and which item names it, read of the use rust-js's
//! `jsx!` pass writes beside its struct, which rustc resolved (ADR 1051).

use rustc_hir::def::{DefKind, Res};
use rustc_hir::intravisit::{self, Visitor};
use rustc_middle::ty::{FieldDef, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};
use rustc_span::{Span, Symbol, sym};

/// What a field is in JS, `object.key` or `object[SYMBOL]`: its name, text,
/// or the item holding the symbol that names it. Only `field_key` makes
/// one, so each reader of a field's key asks it which.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct FieldKey(Key);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Key {
    Name(String),
    /// The item, and where the path naming it is written.
    Symbol(DefId, Span),
}

impl FieldKey {
    pub(in crate::lower) fn named(name: String) -> FieldKey {
        FieldKey(Key::Name(name))
    }

    pub(in crate::lower) fn symbol(item: DefId, span: Span) -> FieldKey {
        FieldKey(Key::Symbol(item, span))
    }

    /// Its text, of a field named by text.
    pub(crate) fn name(&self) -> Option<&str> {
        match &self.0 {
            Key::Name(name) => Some(name),
            Key::Symbol(..) => None,
        }
    }

    /// The item holding the symbol that names it, and where its path is
    /// written, which `item_read` reads it as.
    pub(crate) fn symbol_item(&self) -> Option<(DefId, Span)> {
        match &self.0 {
            Key::Name(_) => None,
            Key::Symbol(item, span) => Some((*item, *span)),
        }
    }

    /// Its text, of a field `validate` lets be named by text alone: an enum
    /// variant's, a union's, a class's or a props struct's (ADR 1051).
    pub(crate) fn text(&self) -> &str {
        self.name().expect("`bindings::validate` refuses a symbol name here")
    }
}

/// Whether `field` has a `#[rust_js::name(..)]` of a path, a symbol's,
/// where `#[rust_js::name = ".."]` names it by text.
pub(crate) fn named_by_symbol(tcx: TyCtxt<'_>, field: DefId) -> bool {
    tcx.get_attrs_by_path(field, &[Symbol::intern("rust_js"), sym::name])
        .any(|attr| attr.meta_item_list().is_some())
}

/// The item `#[rust_js::name(SYMBOL)]` names `field` by, and where its path
/// is written: what its use beside its struct, `|value|
/// ::js::symbol::names_field(&value.field, &SYMBOL)`, resolves `SYMBOL` to,
/// a static, a thread-local or a constant. `None` of a field without one,
/// or whose struct has none written, one of type parameters or in a
/// function's body, which `validate` refuses.
pub(crate) fn symbol_name(tcx: TyCtxt<'_>, field: &FieldDef) -> Option<(DefId, Span)> {
    if !named_by_symbol(tcx, field.did) {
        return None;
    }
    let local = field.did.as_local()?;
    let module = tcx.parent_module_from_def_id(local);
    (tcx.hir_module_items(module).definitions())
        .filter(|&def| is_names_field(tcx, def))
        .find_map(|def| named_in(tcx, def, field.did))
}

/// Whether `def` is a use of a field's symbol name the pass wrote.
fn is_names_field(tcx: TyCtxt<'_>, def: LocalDefId) -> bool {
    matches!(tcx.def_kind(def), DefKind::Const { .. })
        && tcx
            .get_attrs_by_path(
                def.to_def_id(),
                &[Symbol::intern("rust_js"), Symbol::intern("names_field")],
            )
            .next()
            .is_some()
}

/// Of the use `companion`, the item it names `field` by, if it's `field`'s.
fn named_in(tcx: TyCtxt<'_>, companion: LocalDefId, field: DefId) -> Option<(DefId, Span)> {
    struct Find<'tcx> {
        tcx: TyCtxt<'tcx>,
        field: DefId,
        found: Option<(DefId, Span)>,
    }
    impl<'tcx> Visitor<'tcx> for Find<'tcx> {
        type NestedFilter = rustc_middle::hir::nested_filter::OnlyBodies;
        fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
            self.tcx
        }
        fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr<'tcx>) {
            use rustc_hir::ExprKind::{AddrOf, Call, Field, Path};
            if let Call(_, [named, symbol]) = expr.kind
                && let AddrOf(_, _, named) = named.kind
                && let Field(base, _) = named.kind
                && let AddrOf(_, _, symbol) = symbol.kind
                && let Path(qpath) = &symbol.kind
            {
                let results = self.tcx.typeck(expr.hir_id.owner.def_id);
                let index = results.field_index(named.hir_id);
                let base = results.expr_ty_adjusted(base).peel_refs();
                if let rustc_middle::ty::Adt(adt, _) = base.kind()
                    && adt.non_enum_variant().fields[index].did == self.field
                    && let Res::Def(DefKind::Static { .. } | DefKind::Const { .. }, item) =
                        results.qpath_res(qpath, symbol.hir_id)
                {
                    self.found = Some((item, symbol.span));
                }
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut find = Find {
        tcx,
        field,
        found: None,
    };
    find.visit_body(tcx.hir_body_owned_by(companion));
    find.found
}

/// Whether `item` names a field of the crate's, a symbol a
/// `#[rust_js::name(..)]` resolves to: its declaration's type is a `unique
/// symbol`, which TypeScript takes a computed key of.
pub(crate) fn names_a_field(tcx: TyCtxt<'_>, item: DefId) -> bool {
    (tcx.hir_crate_items(()).definitions())
        .filter(|&def| matches!(tcx.def_kind(def), DefKind::Struct))
        .flat_map(|def| tcx.adt_def(def).all_fields())
        .filter_map(|field| symbol_name(tcx, field))
        .any(|(named, _)| named == item)
}

/// The crate's items and its structs', enums' and unions' fields, each a
/// definition a `#[rust_js::name]` may be written on: a field is no
/// definition `hir_crate_items` lists of its own.
fn named_or_fields(tcx: TyCtxt<'_>) -> Vec<DefId> {
    let mut defs = Vec::new();
    for def in tcx.hir_crate_items(()).definitions() {
        defs.push(def.to_def_id());
        if matches!(tcx.def_kind(def), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            defs.extend(tcx.adt_def(def).all_fields().map(|field| field.did));
        }
    }
    defs
}

/// Each `#[rust_js::name(SYMBOL)]` that can't name its field by a symbol,
/// refused, saying what to write; whether there's none (ADR 1051).
pub(crate) fn validate(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    let mut refuse = |span: Span, why: &str| {
        tcx.dcx()
            .span_err(span, format!("rust-js: `#[rust_js::name(SYMBOL)]` {why} (ADR 1051)"));
        valid = false;
    };
    // The structs a component takes as its props, whose fields are JSX's
    // attributes, each a name.
    let props: Vec<DefId> = (tcx.hir_crate_items(()).definitions())
        .filter(|&def| matches!(tcx.def_kind(def), DefKind::Fn) && super::is_component(tcx, def.to_def_id()))
        .filter_map(|def| {
            let sig = tcx.fn_sig(def).instantiate_identity().skip_binder();
            match sig.inputs().first()?.peel_refs().kind() {
                rustc_middle::ty::Adt(adt, _) => Some(adt.did()),
                _ => None,
            }
        })
        .collect();
    for def_id in named_or_fields(tcx) {
        for attr in tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), sym::name]) {
            let Some(list) = attr.meta_item_list() else { continue };
            let span = attr.span();
            if tcx.def_kind(def_id) != DefKind::Field {
                refuse(
                    span,
                    "names a struct's field alone: an item or a variant is named by text, `#[rust_js::name = \"..\"]`",
                );
                continue;
            }
            if !matches!(list.as_slice(), [one] if one.meta_item().is_some_and(|item| item.is_word())) {
                refuse(
                    span,
                    "names its field by one symbol, the path of the item holding it: `#[rust_js::name(RESPONSE_SYMBOL)]`",
                );
                continue;
            }
            let parent = tcx.parent(def_id);
            if tcx.def_kind(parent) != DefKind::Struct {
                refuse(
                    span,
                    "names a struct's field, not an enum variant's or a union's, whose keys are its own: name it by text, `#[rust_js::name = \"..\"]`",
                );
                continue;
            }
            let adt = tcx.adt_def(parent);
            let Some(field) = adt.all_fields().find(|field| field.did == def_id) else {
                continue;
            };
            if symbol_name(tcx, field).is_none() {
                refuse(
                    span,
                    "names a field of a struct written in a module, of no type or const parameter: rust-js reads which symbol it is of the use it writes beside such a struct",
                );
                continue;
            }
            let refused = if crate::lower::classes::is_class(tcx, parent)
                || crate::lower::classes::is_object(tcx, parent)
            {
                Some("names no field of a class or an object of methods, which are `this`'s".to_owned())
            } else if super::is_flatten(tcx, field) {
                Some("names no flattened field, whose own fields are its parent's".to_owned())
            } else if super::is_test_field(tcx, def_id) {
                Some("names no `#[rust_js::test]` member, which an untagged enum's test reads by its name".to_owned())
            } else if props.contains(&parent) {
                Some("names no component's props field: JSX's attributes are text".to_owned())
            } else {
                reading_trait(tcx, parent).map(|name| {
                    format!(
                        "names no field of a struct that derives or implements `{name}`, which reads each field by its name: write `{name}` for it otherwise, or name the field by text"
                    )
                })
            };
            if let Some(refused) = refused {
                refuse(span, &refused);
            }
        }
    }
    valid
}

/// A trait the crate derives or implements for `adt` whose JS reads each
/// field by its name, which no symbol name is yet: `Clone`, `PartialEq`,
/// `PartialOrd`, `Debug`, `Hash` and their kin, and serde's.
fn reading_trait(tcx: TyCtxt<'_>, adt: DefId) -> Option<String> {
    use crate::lower::recognition::{StdItem, is_std_def};
    use rustc_hir::attrs::lang_items::LangItem;
    let reads = |trait_id: DefId| {
        [
            LangItem::Clone,
            LangItem::Copy,
            LangItem::PartialEq,
            LangItem::PartialOrd,
        ]
        .into_iter()
        .any(|item| tcx.is_lang_item(trait_id, item))
            || [StdItem::Debug, StdItem::Eq, StdItem::Ord, StdItem::Hash]
                .into_iter()
                .any(|item| is_std_def(tcx, trait_id, item))
            || crate::lower::serde::serde_trait(tcx, trait_id).is_some()
    };
    for (&trait_id, impls) in tcx.all_local_trait_impls(()) {
        if !reads(trait_id) {
            continue;
        }
        let of_adt = impls.iter().any(|&imp| {
            matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                rustc_middle::ty::Adt(of, _) if of.did() == adt)
        });
        if of_adt {
            return Some(tcx.item_name(trait_id).to_string());
        }
    }
    None
}
