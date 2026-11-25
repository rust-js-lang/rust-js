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
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{CRATE_DEF_ID, DefId};
use rustc_span::kw;

use rust_js_program::offered::{OfferedKind, OfferedPath, OfferedType};

/// What the crate offers JS, in the order its root's children are.
pub fn offered_paths(tcx: TyCtxt<'_>, foreign: &crate::library::Foreign<'_, '_>) -> Vec<OfferedPath> {
    let mut paths = Vec::new();
    walk(tcx, foreign, (CRATE_DEF_ID.to_def_id(), &[]), (&[], &[]), &mut paths);
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
fn walk(
    tcx: TyCtxt<'_>,
    foreign: &crate::library::Foreign<'_, '_>,
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
            bounds: Vec::new(),
        };
        match kind {
            // A module of its own JS reads only through a re-export of it.
            DefKind::Mod if child.reexport_chain.is_empty() => {}
            DefKind::Mod if !id.is_local() && foreign.module_file(id).is_none() => {}
            DefKind::Mod if within.contains(&id) => {}
            DefKind::Mod => {
                paths.push(offered(OfferedKind::Namespace, Vec::new(), None, false));
                let within: Vec<DefId> = within.iter().copied().chain([id]).collect();
                walk(tcx, foreign, (id, &within), (&rust_path, &js_path), paths);
            }
            DefKind::Fn => {
                let signature = tcx
                    .instantiate_bound_regions_with_erased(tcx.fn_sig(id).instantiate_identity().skip_normalization());
                let params = signature
                    .inputs()
                    .iter()
                    .map(|&param| offered_type(tcx, param))
                    .collect();
                let generic = tcx.generics_of(id).requires_monomorphization(tcx);
                let located = crate::recognition::tracks_caller(tcx, id);
                let bounds = bounds_of(tcx, id);
                let function = offered(
                    OfferedKind::Function,
                    params,
                    Some(offered_type(tcx, signature.output())),
                    generic,
                );
                paths.push(OfferedPath {
                    located,
                    bounds,
                    ..function
                });
            }
            // A thread-local's key, whose JS is its module's variable, its own
            // (ADR 0270): no value JS reads of it.
            DefKind::Const | DefKind::Static { .. } if crate::bindings::thread_local_value(tcx, id).is_some() => {}
            DefKind::Const | DefKind::Static { .. } => {
                let of = offered_type(tcx, tcx.type_of(id).instantiate_identity().skip_normalization());
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
                let (fields, sealed) = match kind {
                    DefKind::Struct => fields_of(tcx, id),
                    _ => (Vec::new(), false),
                };
                paths.push(OfferedPath {
                    type_params,
                    of_no_value,
                    fields,
                    sealed,
                    ..offered(OfferedKind::Type, Vec::new(), None, false)
                });
            }
            _ => {}
        }
    }
}

/// `ty` as the export net gives values of it: a plain form, or its name.
fn offered_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> OfferedType {
    let of = |ty: Ty<'tcx>| Box::new(offered_type(tcx, ty));
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
            items: items.iter().map(|item| offered_type(tcx, item)).collect(),
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
                ty::Tuple(items) => items.iter().map(|item| offered_type(tcx, item)).collect(),
                _ => Vec::new(),
            };
            let output = (predicates.projection_bounds().next())
                .and_then(|projection| tcx.instantiate_bound_regions_with_erased(projection).term.as_type())
                .map_or(OfferedType::Unit, |ty| offered_type(tcx, ty));
            OfferedType::Fn {
                inputs,
                output: Box::new(output),
            }
        }
        ty::Param(param) => OfferedType::Param {
            name: param.name.to_string(),
        },
        ty::Adt(adt, _) if adt.did().is_local() => OfferedType::Named {
            rust: tcx.def_path_str(adt.did()).split("::").map(str::to_string).collect(),
        },
        _ => OfferedType::Other { rust: ty.to_string() },
    }
}

/// A struct's named fields, each by its Rust name, its JS name, its type and
/// its type as written; and whether it's sealed (`OfferedPath::sealed`).
fn fields_of(tcx: TyCtxt<'_>, id: DefId) -> (Vec<rust_js_program::offered::OfferedField>, bool) {
    let variant = tcx.adt_def(id).non_enum_variant();
    // Of named fields, each named by text: a field a JS symbol names, a
    // binding's, has no name a caller's literal gives (ADR 1051).
    if variant.ctor.is_some()
        || (variant.fields.iter()).any(|field| crate::bindings::field_key(tcx, field).name().is_none())
    {
        return (Vec::new(), false);
    }
    let fields = (variant.fields.iter().enumerate())
        .map(|(i, field)| {
            let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
            rust_js_program::offered::OfferedField {
                rust: field.name.to_string(),
                js: crate::representation::variant_field(tcx, variant, i),
                ty: offered_type(tcx, ty),
                written: ty.to_string(),
            }
        })
        .collect();
    let sealed = tcx.visibility(id).is_public()
        && !crate::classes::is_class(tcx, id)
        && variant.fields.iter().any(|field| !field.vis.is_public());
    (fields, sealed)
}

/// What a generic function asks of each of its type parameters, by rustc's
/// predicates of it: the traits it's bound by; of a callable bound, the
/// function it is, its `Output` by the predicate's projection; and of a
/// trait of the crate's, each impl of it rustc has (`all_impls`), by the
/// type it's of and its dictionary's JS name (`traits::words::impl_name`).
fn bounds_of(tcx: TyCtxt<'_>, id: DefId) -> Vec<rust_js_program::offered::OfferedBound> {
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
                        ty::Tuple(items) => items.iter().map(|item| offered_type(tcx, item)).collect(),
                        _ => Vec::new(),
                    };
                    let output = (predicates.iter())
                        .filter_map(|p| p.as_projection_clause())
                        .map(|p| tcx.instantiate_bound_regions_with_erased(p))
                        .find(|p| of_param(p.projection_term.self_ty()))
                        .and_then(|p| p.term.as_type())
                        .map_or(OfferedType::Unit, |ty| offered_type(tcx, ty));
                    callable = Some(OfferedType::Fn {
                        inputs,
                        output: Box::new(output),
                    });
                }
                if tr.def_id.is_local() {
                    for imp in tcx.all_impls(tr.def_id).filter(|imp| imp.is_local()) {
                        let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
                        impls.push(rust_js_program::offered::OfferedImpl {
                            ty: offered_type(tcx, self_ty),
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
