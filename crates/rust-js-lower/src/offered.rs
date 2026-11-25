//! The paths a crate offers JS (#2481): from its root, each `pub` item and
//! each `pub use` of it, and through each module it re-exports, the crate's
//! or a library's, what that module offers in turn, by its Rust path and
//! the path JS reads it by (`manifest::Manifest::offered_paths`). Found of
//! rustc's `module_children` alone, apart from what writes the JS and its
//! `.d.ts`: a re-export the writer drops is a path the JS lacks (#2544),
//! not one both agree is gone. A module's own `pub mod` isn't one, as only
//! Rust names it (#2403 lever (3)); nor a library's module that has no JS,
//! of types alone or a binding crate's.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::middle::resolve::Reexport;
use rustc_middle::ty::consts::ConstExt as _;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{CRATE_DEF_ID, DefId};
use rustc_span::kw;

use crate::recognition::StdItem;
use rust_js_program::offered::{OfferedKind, OfferedPath, OfferedType};

/// What the records ask besides rustc, each of its owner: a library's
/// contract (`Foreign`), the crate's keyed enums (`Flattened`), its counted
/// `Rc`s (`Counted`) and how it holds its `RefCell`s (`RefCells`).
pub struct Facts<'a, 'tcx> {
    pub foreign: &'a crate::library::Foreign<'a, 'tcx>,
    pub flattened: &'a crate::bindings::Flattened,
    pub counted: &'a crate::analysis::Counted<'tcx>,
    pub ref_cells: &'a crate::drops::RefCells<'tcx>,
}

/// What the crate offers JS, in the order its root's children are.
pub fn offered_paths<'tcx>(tcx: TyCtxt<'tcx>, facts: &Facts<'_, 'tcx>) -> Vec<OfferedPath> {
    let mut paths = Vec::new();
    walk(tcx, facts, (CRATE_DEF_ID.to_def_id(), &[]), (&[], &[]), &mut paths);
    paths
}

/// A module's children, as rustc resolves them: its items and its `use`s'.
fn children(tcx: TyCtxt<'_>, module: DefId) -> &[rustc_middle::middle::resolve::ModChild] {
    match module.as_local() {
        Some(local) => tcx.module_children_local(local),
        None => tcx.module_children(module),
    }
}

/// What `module`, at `rust`'s and `js`'s paths, offers, each pushed to
/// `paths`; not a module `within` it already, which re-exports itself.
fn walk<'tcx>(
    tcx: TyCtxt<'tcx>,
    facts: &Facts<'_, 'tcx>,
    (module, within): (DefId, &[DefId]),
    (rust, js): (&[String], &[String]),
    paths: &mut Vec<OfferedPath>,
) {
    for child in children(tcx, module) {
        let ty::Visibility::Public = child.vis else { continue };
        let rustc_hir::def::Res::Def(kind, id) = child.res.expect_non_local::<!>() else {
            continue;
        };
        let name = child.ident.name;
        // What JS has a value or a type of; not a macro, a trait or a
        // variant, which it hasn't.
        let offers = matches!(
            kind,
            DefKind::Mod
                | DefKind::Fn
                | DefKind::Const
                | DefKind::Static { .. }
                | DefKind::Struct
                | DefKind::Enum
                | DefKind::Union
                | DefKind::TyAlias
        );
        if name == kw::Underscore || !offers {
            continue;
        }
        // By the name its `use` gives it, as JS reads a re-export; else its own.
        let js_name = match child.reexport_chain.first() {
            Some(&(Reexport::Single(used) | Reexport::ExternCrate(used))) => {
                crate::bindings::alias_name(tcx, used, name)
            }
            _ if kind == DefKind::Mod => rust_js_program::casing::camel_case(name.as_str()),
            _ => crate::bindings::fn_name(tcx, id),
        };
        let path = |of: &[String], last: String| of.iter().cloned().chain(std::iter::once(last)).collect::<Vec<_>>();
        let (rust_path, js_path) = (path(rust, name.to_string()), path(js, js_name));
        let offered = |kind, params, ty, generic| OfferedPath {
            rust: rust_path.clone(),
            js: js_path.clone(),
            kind,
            params,
            ty,
            generic,
            type_params: 0,
            of_no_value: false,
            located: false,
            fields: Vec::new(),
            sealed: false,
            unoffered_fields: None,
            bounds: Vec::new(),
            result_none: rust_js_program::offered::OfferedNone::Plain,
            def: None,
            transparent: None,
            variants: None,
        };
        match kind {
            // A module of its own JS reads only through a re-export of it.
            DefKind::Mod if child.reexport_chain.is_empty() => {}
            DefKind::Mod if !id.is_local() && facts.foreign.module_file(id).is_none() => {}
            DefKind::Mod if within.contains(&id) => {}
            DefKind::Mod => {
                paths.push(offered(OfferedKind::Namespace, Vec::new(), None, false));
                let within: Vec<DefId> = within.iter().copied().chain([id]).collect();
                walk(tcx, facts, (id, &within), (&rust_path, &js_path), paths);
            }
            DefKind::Fn => {
                let signature = tcx
                    .instantiate_bound_regions_with_erased(tcx.fn_sig(id).instantiate_identity().skip_normalization());
                let params = signature
                    .inputs()
                    .iter()
                    .map(|&param| offered_type(tcx, facts, param))
                    .collect();
                let generic = tcx.generics_of(id).requires_monomorphization(tcx);
                let located = crate::recognition::tracks_caller(tcx, id);
                let bounds = bounds_of(tcx, facts, id);
                let function = offered(
                    OfferedKind::Function,
                    params,
                    Some(offered_type(tcx, facts, signature.output())),
                    generic,
                );
                // Its result's `None`, as its `#[rust_js::nullable]` says.
                let result_none =
                    match crate::representation::none_form(tcx, crate::representation::NonePlace::FnResult(id)) {
                        crate::representation::NoneForm::Null => rust_js_program::offered::OfferedNone::Null,
                        _ => rust_js_program::offered::OfferedNone::Plain,
                    };
                paths.push(OfferedPath {
                    located,
                    bounds,
                    result_none,
                    ..function
                });
            }
            // A thread-local's key, whose JS is its module's variable, its own
            // (ADR 0270): no value JS reads of it.
            DefKind::Const | DefKind::Static { .. } if crate::bindings::thread_local_value(tcx, id).is_some() => {}
            DefKind::Const | DefKind::Static { .. } => {
                let of = offered_type(tcx, facts, tcx.type_of(id).instantiate_identity().skip_normalization());
                let kind = match kind {
                    DefKind::Static { .. } => OfferedKind::Static,
                    _ => OfferedKind::Const,
                };
                paths.push(offered(kind, Vec::new(), Some(of), false));
            }
            DefKind::Struct | DefKind::Enum | DefKind::Union | DefKind::TyAlias => {
                let type_params = tcx.generics_of(id).own_counts().types;
                // A struct of no field but `PhantomData`s holds no value.
                let of_no_value = kind == DefKind::Struct
                    && !(tcx.adt_def(id).non_enum_variant().fields.iter()).any(|field| {
                        let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                        !matches!(ty.kind(), ty::Adt(adt, _) if adt.is_phantom_data())
                    });
                let (fields, sealed, unoffered_fields) = match kind {
                    DefKind::Struct => fields_of(tcx, facts, id),
                    _ => (Vec::new(), false, None),
                };
                // A transparent struct's JS, its field's.
                let transparent = (kind == DefKind::Struct).then(|| tcx.adt_def(id)).and_then(|adt| {
                    crate::recognition::transparent_field(tcx, adt).map(|i| {
                        let field = &adt.non_enum_variant().fields[rustc_abi::FieldIdx::from_usize(i)];
                        offered_type(
                            tcx,
                            facts,
                            tcx.type_of(field.did).instantiate_identity().skip_normalization(),
                        )
                    })
                });
                paths.push(OfferedPath {
                    type_params,
                    of_no_value,
                    fields,
                    sealed,
                    unoffered_fields,
                    def: Some(def_of(tcx, id)),
                    transparent,
                    variants: (kind == DefKind::Enum).then(|| enum_of(tcx, facts, id)),
                    ..offered(OfferedKind::Type, Vec::new(), None, false)
                });
            }
            _ => {}
        }
    }
}

/// `ty` as the export net gives values of it: a plain form, or its name.
fn offered_type<'tcx>(tcx: TyCtxt<'tcx>, facts: &Facts<'_, 'tcx>, ty: Ty<'tcx>) -> OfferedType {
    let of = |ty: Ty<'tcx>| Box::new(offered_type(tcx, facts, ty));
    match ty.kind() {
        ty::Int(int) => OfferedType::Int {
            rust: int.name_str().to_string(),
        },
        ty::Uint(int) => OfferedType::Int {
            rust: int.name_str().to_string(),
        },
        ty::Float(float) => OfferedType::Float {
            rust: float.name_str().to_string(),
        },
        ty::Bool => OfferedType::Bool,
        ty::Char => OfferedType::Char,
        ty::Tuple(items) if items.is_empty() => OfferedType::Unit,
        ty::Tuple(items) => OfferedType::Tuple {
            items: items.iter().map(|item| offered_type(tcx, facts, item)).collect(),
        },
        // A `&mut`, a box or the object it points at (ADR 1086), as JS gives
        // one: no value of this net's.
        ty::Ref(_, inner, rustc_middle::ty::Mutability::Mut) => OfferedType::Ref {
            of: of(*inner),
            mutable: true,
        },
        ty::Ref(_, inner, _) => match inner.kind() {
            ty::Str => OfferedType::Str { owned: false },
            ty::Slice(item) => OfferedType::Vec {
                of: of(*item),
                borrowed: true,
            },
            _ => OfferedType::Ref {
                of: of(*inner),
                mutable: false,
            },
        },
        ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String) => OfferedType::Str { owned: true },
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), LangItem::Option) => OfferedType::Option {
            of: of(args.type_at(0)),
        },
        ty::Adt(adt, args) if crate::recognition::is_std_def(tcx, adt.did(), crate::recognition::StdItem::Vec) => {
            OfferedType::Vec {
                of: of(args.type_at(0)),
                borrowed: false,
            }
        }
        ty::Adt(_, args) if ty.is_box() => OfferedType::Box {
            of: of(args.type_at(0)),
        },
        // A `dyn Fn(u32) -> u32`, the function it is: its inputs of its
        // trait's, its output of its projection.
        ty::Dynamic(predicates, ..)
            if let Some(principal) = predicates.principal()
                && tcx.is_fn_trait(principal.def_id()) =>
        {
            let principal = tcx.instantiate_bound_regions_with_erased(principal);
            let inputs = match principal.args.type_at(0).kind() {
                ty::Tuple(items) => items.iter().map(|item| offered_type(tcx, facts, item)).collect(),
                _ => Vec::new(),
            };
            let output = (predicates.projection_bounds().next())
                .and_then(|projection| tcx.instantiate_bound_regions_with_erased(projection).term.as_type())
                .map_or(OfferedType::Unit, |ty| offered_type(tcx, facts, ty));
            OfferedType::Fn {
                inputs,
                output: Box::new(output),
            }
        }
        ty::Adt(adt, args) if crate::recognition::is_std_def(tcx, adt.did(), StdItem::Result) => OfferedType::Result {
            ok: of(args.type_at(0)),
            err: of(args.type_at(1)),
        },
        ty::Array(item, len) => OfferedType::Array {
            of: of(*item),
            len: len.try_to_target_usize(tcx).unwrap_or(0),
        },
        // An `Rc`, counted where the crate counts it, the instances' answer.
        ty::Adt(adt, args) if crate::recognition::is_std_def(tcx, adt.did(), StdItem::Rc) => OfferedType::Rc {
            of: of(args.type_at(0)),
            counted: facts.counted.counts(tcx, args.type_at(0)),
        },
        ty::Adt(adt, args) if crate::recognition::is_std_def(tcx, adt.did(), StdItem::Arc) => OfferedType::Box {
            of: of(args.type_at(0)),
        },
        ty::Adt(adt, args) if crate::recognition::is_std_def(tcx, adt.did(), StdItem::Cell) => OfferedType::Cell {
            of: of(args.type_at(0)),
            itself: false,
        },
        // A `RefCell`, the object it holds where the crate holds it so.
        ty::Adt(adt, args) if crate::recognition::is_std_def(tcx, adt.did(), StdItem::RefCell) => OfferedType::Cell {
            of: of(args.type_at(0)),
            itself: facts.ref_cells.is_value_cell(tcx, ty),
        },
        // A `fn` pointer, the function it is.
        ty::FnPtr(..) => {
            let signature = tcx.instantiate_bound_regions_with_erased(ty.fn_sig(tcx));
            OfferedType::Fn {
                inputs: signature
                    .inputs()
                    .iter()
                    .map(|&input| offered_type(tcx, facts, input))
                    .collect(),
                output: of(signature.output()),
            }
        }
        // A `dyn` trait not of a function, by its traits.
        ty::Dynamic(predicates, ..) => OfferedType::Dyn {
            traits: (predicates.principal_def_id().into_iter())
                .chain(predicates.auto_traits())
                .map(|tr| tcx.def_path_str(tr))
                .collect(),
        },
        // An `impl Trait` given back, as its bounds say: an iterator, or a
        // function.
        ty::Alias(
            _,
            ty::AliasTy {
                kind: ty::Opaque { def_id },
                ..
            },
        ) => opaque(tcx, facts, *def_id).unwrap_or_else(|| OfferedType::Other { rust: ty.to_string() }),
        // One of std's iterators, `Chars`: a JS iterator.
        ty::Adt(adt, _)
            if crate::recognition::is_std_crate(tcx, adt.did())
                && crate::recognition::trait_holds_in(
                    tcx,
                    ty::TypingEnv::fully_monomorphized(),
                    ty,
                    crate::recognition::std_item(tcx, StdItem::Iterator),
                ) == Some(true) =>
        {
            OfferedType::Iterator {
                back: false,
                len: false,
            }
        }
        ty::Param(param) => OfferedType::Param {
            name: param.name.to_string(),
            index: param.index,
        },
        // Each of its generic arguments by its index, which its own record's
        // `Param`s name: a lifetime's, which holds no value, `Other`.
        ty::Adt(adt, args) if !crate::recognition::is_std_crate(tcx, adt.did()) => OfferedType::Named {
            def: def_of(tcx, adt.did()),
            args: (args.iter())
                .map(|arg| match arg.as_type() {
                    Some(ty) => offered_type(tcx, facts, ty),
                    None => OfferedType::Other { rust: arg.to_string() },
                })
                .collect(),
        },
        _ => OfferedType::Other { rust: ty.to_string() },
    }
}

/// How JS holds an enum's values, each the owner's answer: untagged
/// (`is_untagged`), keyed (`Flattened::keyed`), of a declared tag
/// (`declared_tag`) or tagged, its tag's key (`tag_key`), and each variant's
/// tag (`variant_tag`, or an untagged one's of no fields `unit_value`), its
/// fields by their JS names (`variant_field`) and whether it's an
/// `otherwise` (`is_otherwise`).
fn enum_of<'tcx>(tcx: TyCtxt<'tcx>, facts: &Facts<'_, 'tcx>, id: DefId) -> rust_js_program::offered::OfferedEnum {
    use rust_js_program::js::ExprKind;
    use rust_js_program::offered::{OfferedEnum, OfferedEnumForm, OfferedField, OfferedTag, OfferedVariant};
    let adt = tcx.adt_def(id);
    let untagged = crate::bindings::is_untagged(tcx, id);
    let form = if untagged {
        OfferedEnumForm::Untagged
    } else if facts.flattened.keyed(tcx, adt).is_some() {
        OfferedEnumForm::Keyed
    } else if crate::bindings::declared_tag(tcx, id).is_some() {
        OfferedEnumForm::Declared
    } else {
        OfferedEnumForm::Tagged
    };
    let tag_of = |expr: rust_js_program::js::Expr| match expr.kind {
        ExprKind::Null => OfferedTag::Null,
        ExprKind::Undefined => OfferedTag::Undefined,
        ExprKind::Bool(b) => OfferedTag::Bool(b),
        ExprKind::Num(n) => OfferedTag::Num(n),
        ExprKind::Str(s) => OfferedTag::Str(s.to_string()),
        _ => OfferedTag::Str(String::new()),
    };
    let variants = (adt.variants().iter())
        .map(|variant| {
            let tag = match untagged && variant.fields.is_empty() {
                true => crate::untagged::unit_value(tcx, variant).map_or(OfferedTag::Str(String::new()), tag_of),
                false => tag_of(crate::bindings::variant_tag(tcx, variant)),
            };
            let fields = (variant.fields.iter().enumerate())
                .map(|(i, field)| {
                    let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    OfferedField {
                        rust: field.name.to_string(),
                        js: crate::representation::variant_field(tcx, variant, i),
                        ty: offered_type(tcx, facts, ty),
                        written: ty.to_string(),
                        none: none_of(tcx, field),
                        property: crate::representation::property_of(tcx, facts.ref_cells, Some(field), ty)
                            .map(|property| offered_type(tcx, facts, property)),
                    }
                })
                .collect();
            OfferedVariant {
                rust: variant.name.to_string(),
                tag,
                fields,
                otherwise: crate::bindings::is_otherwise(tcx, variant.def_id),
            }
        })
        .collect();
    OfferedEnum {
        form,
        key: crate::bindings::tag_key(tcx, id),
        variants,
    }
}

/// An `impl Trait` given back, as its bounds say: an `Iterator`, its
/// `back()` of a `DoubleEndedIterator` bound, its `len()` of an
/// `ExactSizeIterator` one; or an `Fn`, `FnMut` or `FnOnce`, the function
/// its bound and its `Output` projection say. `None` of any other.
fn opaque<'tcx>(tcx: TyCtxt<'tcx>, facts: &Facts<'_, 'tcx>, id: DefId) -> Option<OfferedType> {
    let bounds: Vec<_> = (tcx.explicit_item_bounds(id).iter_identity_copied())
        .map(|clause| clause.skip_normalization().0)
        .collect();
    let traits: Vec<DefId> = (bounds.iter())
        .filter_map(|clause| clause.as_trait_clause())
        .map(|tr| tr.def_id())
        .collect();
    // Of a bound or a supertrait of one: `impl DoubleEndedIterator`'s
    // `Iterator`, which no clause of its own says.
    let has = |tr: Option<DefId>| {
        tr.is_some_and(|tr| {
            (traits.iter())
                .any(|&bound| bound == tr || ty::elaborate::supertrait_def_ids(tcx, bound).any(|sup| sup == tr))
        })
    };
    if has(Some(crate::recognition::std_item(tcx, StdItem::Iterator))) {
        return Some(OfferedType::Iterator {
            back: has(Some(crate::recognition::std_item(tcx, StdItem::DoubleEndedIterator))),
            len: has(crate::recognition::exact_size_iterator_trait(tcx)),
        });
    }
    let callable = (bounds.iter())
        .filter_map(|clause| clause.as_trait_clause())
        .find(|tr| tcx.is_fn_trait(tr.def_id()))?;
    let callable = tcx.instantiate_bound_regions_with_erased(callable);
    let inputs = match callable.trait_ref.args.type_at(1).kind() {
        ty::Tuple(items) => items.iter().map(|item| offered_type(tcx, facts, item)).collect(),
        _ => Vec::new(),
    };
    let output = (bounds.iter())
        .filter_map(|clause| clause.as_projection_clause())
        .find_map(|projection| tcx.instantiate_bound_regions_with_erased(projection).term.as_type())
        .map_or(OfferedType::Unit, |ty| offered_type(tcx, facts, ty));
    Some(OfferedType::Fn {
        inputs,
        output: Box::new(output),
    })
}

/// Where a struct, an enum or a union is defined, its records' one key:
/// its crate's name and its path from that crate's root.
fn def_of(tcx: TyCtxt<'_>, id: DefId) -> rust_js_program::offered::OfferedDef {
    rust_js_program::offered::OfferedDef {
        krate: crate::library::canonical_crate_name(tcx, id.krate),
        rust: (tcx.def_path(id).data.iter())
            .filter_map(|segment| segment.data.get_opt_name())
            .map(|name| name.to_string())
            .collect(),
    }
}

/// What a field's `None` is, of its owner's one answer, `none_form`, and
/// whether its key is written, `#[rust_js::present]`'s other axis.
fn none_of(tcx: TyCtxt<'_>, field: &rustc_middle::ty::FieldDef) -> rust_js_program::offered::OfferedNone {
    use crate::representation::{NoneForm, NonePlace, none_form};
    use rust_js_program::offered::OfferedNone;
    match none_form(tcx, NonePlace::Field(field, None)) {
        NoneForm::Value => OfferedNone::Held,
        NoneForm::Optional => OfferedNone::NullOptional,
        NoneForm::Null => OfferedNone::Null,
        NoneForm::Nested => OfferedNone::Nested,
        NoneForm::Undefined | NoneForm::Either if crate::bindings::is_present(tcx, field) => OfferedNone::Present,
        NoneForm::Undefined | NoneForm::Either => OfferedNone::Plain,
    }
}

/// A struct's named fields, each by its Rust name, its JS name, its type and
/// its type as written; whether it's sealed (`OfferedPath::sealed`); and
/// of one whose fields aren't recorded, why.
fn fields_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    facts: &Facts<'_, 'tcx>,
    id: DefId,
) -> (
    Vec<rust_js_program::offered::OfferedField>,
    bool,
    Option<rust_js_program::offered::UnofferedFields>,
) {
    let variant = tcx.adt_def(id).non_enum_variant();
    let sealed = tcx.visibility(id).is_public()
        && !crate::classes::is_class(tcx, id)
        && variant.fields.iter().any(|field| !field.vis.is_public());
    // Of named fields alone, a tuple struct's JS its positions.
    if variant.ctor.is_some() {
        return (Vec::new(), sealed, None);
    }
    // A field a JS symbol names (ADR 1051) has no text `js`: the struct
    // says its fields aren't offered, and why.
    if (variant.fields.iter()).any(|field| crate::bindings::field_key(tcx, field).name().is_none()) {
        return (
            Vec::new(),
            sealed,
            Some(rust_js_program::offered::UnofferedFields::SymbolKeyed),
        );
    }
    let fields = (variant.fields.iter().enumerate())
        .map(|(i, field)| {
            let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
            rust_js_program::offered::OfferedField {
                rust: field.name.to_string(),
                js: crate::representation::variant_field(tcx, variant, i),
                ty: offered_type(tcx, facts, ty),
                written: ty.to_string(),
                none: none_of(tcx, field),
                property: crate::representation::property_of(tcx, facts.ref_cells, Some(field), ty)
                    .map(|property| offered_type(tcx, facts, property)),
            }
        })
        .collect();
    (fields, sealed, None)
}

/// What a generic function asks of each of its type parameters, by rustc's
/// predicates of it: the traits it's bound by; of a callable bound, the
/// function it is, its `Output` by the predicate's projection; and of a
/// trait of the crate's, each impl of it rustc has (`all_impls`), by the
/// type it's of and its dictionary's JS name (`traits::words::impl_name`).
fn bounds_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    facts: &Facts<'_, 'tcx>,
    id: DefId,
) -> Vec<rust_js_program::offered::OfferedBound> {
    let generics = tcx.generics_of(id);
    let predicates: Vec<_> = tcx.param_env(id).caller_bounds().collect();
    let mut bounds = Vec::new();
    for param in &generics.own_params {
        let rustc_middle::ty::GenericParamDefKind::Type { .. } = param.kind else {
            continue;
        };
        let of_param = |ty: Ty<'_>| matches!(ty.kind(), ty::Param(p) if p.index == param.index);
        let mut traits = Vec::new();
        let mut callable = None;
        let mut impls = Vec::new();
        for predicate in &predicates {
            if let Some(trait_predicate) = predicate.as_trait_clause() {
                let tr = tcx.instantiate_bound_regions_with_erased(trait_predicate).trait_ref;
                if !of_param(tr.self_ty()) {
                    continue;
                }
                traits.push(tcx.def_path_str(tr.def_id));
                if tcx.is_fn_trait(tr.def_id) {
                    let inputs = match tr.args.type_at(1).kind() {
                        ty::Tuple(items) => items.iter().map(|item| offered_type(tcx, facts, item)).collect(),
                        _ => Vec::new(),
                    };
                    let output = (predicates.iter())
                        .filter_map(|p| p.as_projection_clause())
                        .map(|p| tcx.instantiate_bound_regions_with_erased(p))
                        .find(|p| of_param(p.projection_term.self_ty()))
                        .and_then(|p| p.term.as_type())
                        .map_or(OfferedType::Unit, |ty| offered_type(tcx, facts, ty));
                    callable = Some(OfferedType::Fn {
                        inputs,
                        output: Box::new(output),
                    });
                }
                if tr.def_id.is_local() {
                    for imp in tcx.all_impls(tr.def_id).filter(|imp| imp.is_local()) {
                        let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
                        impls.push(rust_js_program::offered::OfferedImpl {
                            ty: offered_type(tcx, facts, self_ty),
                            dictionary: crate::traits::impl_name(tcx, imp),
                        });
                    }
                }
            }
        }
        bounds.push(rust_js_program::offered::OfferedBound {
            param: param.name.to_string(),
            traits,
            callable,
            impls,
        });
    }
    bounds
}
