//! What's written only where something reads it: derived impls, a marker
//! trait's dictionaries, those only generic code reads, a `Hash`'s among
//! them, a `PartialEq` whose `==` is `===`, a library's the crate compiles
//! itself, and a fieldless enum's own names of the crate alone, only called.

use super::super::Body;
use super::super::library::Foreign;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{HashMap, HashSet};

/// The types equal only to themselves, by their `==`'s keyed form, and
/// those whose `==` is their address's (ADR 0864, amended).
type Identities = (Vec<DefId>, Vec<DefId>);

/// Of the crate's `trait_impls` and its lowered functions, `lowered`: the
/// types equal only to themselves, which its contract lists for its
/// consumers (ADR 0864, amended); and what's written only where it's
/// reached. Derived Debug implementations, a marker trait's dictionaries,
/// which only a `dyn` reads (ADR 0307), and those only generic code reads,
/// their methods called as they are anywhere else (`read_only_generically`),
/// a `Hash`'s and a `Hasher`'s among them, which a map never calls (ADR
/// 0463); a `PartialEq` of a type equal only to itself, whose `==` is `===`
/// and whose sets and maps are JS's (ADR 0864, amended); and a function each
/// call of which is what it's given, a fieldless enum's `name(self)`
/// matching each variant to its name, which no other crate reaches,
/// written where something reads it as a value, as its calls don't (ADR
/// 0264, amended); and each of a library's the crate compiles itself, its
/// impls and its functions, written where the crate's code reaches it
/// (ADR 0914, amended, #483); and the untagged enums each plain `{}` of
/// which is the value in place, their `Display` impls written where reached
/// (rd730).
pub(super) fn written_where_reached<'tcx>(
    tcx: TyCtxt<'tcx>,
    (trait_impls, foreign): (&[DefId], &Foreign<'_, 'tcx>),
    function_bodies: &HashMap<DefId, &Body<'tcx>>,
    lowered: impl Iterator<Item = DefId>,
) -> (Identities, HashSet<DefId>, Vec<DefId>) {
    let recognition = super::super::recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls,
        foreign,
    };
    // A `PartialEq` whose `==` is `===`, whatever its `Hash`; and of a
    // type equal only to itself, whose `Hash` is its address's too.
    let identity_eq = |id: DefId, trait_id: DefId, keyed: bool| {
        let ty = tcx.type_of(id).instantiate_identity().skip_normalization();
        tcx.is_lang_item(trait_id, rustc_hir::attrs::lang_items::LangItem::PartialEq)
            && match keyed {
                true => super::super::std_impls::equal_only_to_itself(&recognition, function_bodies, ty),
                false => super::super::std_impls::eq_by_address(&recognition, function_bodies, ty),
            }
    };
    let trait_of = |id: DefId| {
        tcx.impl_trait_ref(id)
            .instantiate_identity()
            .skip_normalization()
            .def_id
    };
    let of_impls = |keyed: bool| -> Vec<DefId> {
        (trait_impls.iter())
            .filter(|&&id| identity_eq(id, trait_of(id), keyed))
            .filter_map(|&id| tcx.type_of(id).instantiate_identity().skip_normalization().ty_adt_def())
            .map(|adt| adt.did())
            .collect()
    };
    let identities = (of_impls(true), of_impls(false));
    // The untagged enums whose `Display` each plain `{}` writes in place,
    // and their impls (rd730).
    let displaying = super::super::untagged::displayed::displaying_impls(&recognition, function_bodies);
    let derived: HashSet<DefId> = trait_impls
        .iter()
        .filter(|&&id| {
            let trait_id = trait_of(id);
            super::super::recognition::known_derive(tcx, id)
                || super::super::traits::is_marker(tcx, foreign, trait_id)
                || identity_eq(id, trait_id, false)
        })
        .flat_map(|&id| std::iter::once(id).chain(tcx.associated_item_def_ids(id).iter().copied()))
        // And their methods, each called as it is, by its name, where
        // anything calls it: `From<Lane>`'s `from` nothing calls is
        // written nowhere (ADR 0477, amended).
        .chain(
            (trait_impls.iter().copied())
                .filter(|&id| super::super::traits::read_only_generically(tcx, trait_of(id)))
                .flat_map(|id| std::iter::once(id).chain(tcx.associated_item_def_ids(id).iter().copied())),
        )
        // And an untagged enum's `Display` each plain `{}` of which writes
        // the value in place, `${id}`: its impl is written where a call
        // through its dictionary reaches it, a generic one's, and its
        // library's consumers write theirs in place too (rd730).
        .chain(
            (displaying.iter())
                .flat_map(|&(_, imp)| std::iter::once(imp).chain(tcx.associated_item_def_ids(imp).iter().copied())),
        )
        // And what the crate compiles itself of a library, an impl, its
        // methods or a function, only what its code reaches (ADR 0914,
        // amended): a default asked for that a call then made through its
        // library's dictionary isn't (#483).
        // And an untagged enum's method that reads each variant's field,
        // each call of which is that read where its JS is (rd317).
        .chain(lowered.filter(|&id| {
            let field_read = |id| {
                (function_bodies.get(&id))
                    .is_some_and(|body| super::super::untagged::field_reads::reads_each_variants_field(tcx, body))
            };
            foreign.is_instance(id)
                || id.is_local()
                    && !super::super::library::reachable(tcx, id)
                    && super::super::patterns::gives_its_argument(tcx, function_bodies, id)
                || id.is_local() && !super::super::library::reachable(tcx, id) && field_read(id)
        }))
        .collect();
    (
        identities,
        derived,
        displaying.into_iter().map(|(adt, _)| adt).collect(),
    )
}

/// Whether `id`, a body of the crate's, is lowered as a function of its
/// own. Not a trait's default, copied into each impl (ADR 0049), but a
/// library's, which a consumer's impl of its trait calls, a function over
/// `Self`, the consumer's own (ADRs 0185, 0914, amended), and std's,
/// compiled into `@rust-js/std`, its own too (ADR 0561); nor a trait of
/// classes' method that gives a field, which is that field, read in place
/// wherever it's called (rd304, ADR 1038); nor a namespace's method that's
/// the function it passes its parameters on to (`forwarded`, rd292b).
pub(super) fn lowered_here(
    tcx: TyCtxt<'_>,
    (foreign, forwarded): (&Foreign<'_, '_>, &HashMap<DefId, DefId>),
    exports_reachable: bool,
    id: DefId,
) -> bool {
    if super::super::classes::field_accessor(tcx, id).is_some() || forwarded.contains_key(&id) {
        return false;
    }
    tcx.trait_of_assoc(id).is_none()
        || foreign.is_instance(id)
        || (exports_reachable && super::super::library::reachable(tcx, id))
}

/// Each method of the crate that's the function it only passes its
/// parameters on to (`passed_on::forwarded_function`, rd292b, ADR 1037
/// amended), by that function: a namespace's, of another of its modules,
/// one its module re-exports by the method's name, `export {
/// batchedUpdates } from "./ReactFiberWorkLoop.js"`, or of its own, the
/// function its module exports by that name (#566); a prototype's, its
/// property (rd715). Where the two take what JS gives them alike, their
/// parameters and, as they're of the same generic parameters, each given
/// alike, what's given of those: its caller's location of both or neither,
/// and the dictionaries of the same bounds; a type's facts and drops each
/// asks of what it passes on alike. Not one its callers need a copy of
/// (ADR 0051), which is its own. The namespaces' by their functions, which
/// have no JS of their own; and the prototypes', lowered, each its
/// property where its JS is that call alone (`classes::passed_on`).
pub(super) fn forwarded<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    (given, js_given): (super::super::traits::Given<'_>, &super::super::analysis::JsGiven),
    (caller_locations, fns): (&HashSet<DefId>, &HashMap<DefId, super::super::FnInfo>),
    foreign: &Foreign<'_, 'tcx>,
) -> (HashMap<DefId, DefId>, HashMap<DefId, DefId>) {
    let bounds = |id: DefId| super::super::traits::bounds(tcx, foreign, given, id);
    let exported = |id: DefId| fns.get(&id).map(|info| info.exported().to_string());
    let (namespaces, prototypes): (HashMap<DefId, DefId>, HashMap<DefId, DefId>) = (bodies.iter())
        .filter_map(|body| {
            let method = body.def_id;
            let (target, args) = super::super::passed_on::forwarded_function(tcx, method, &body.thir, &exported)?;
            let theirs: Vec<_> = (bounds(target).into_iter())
                .map(|bound| {
                    ty::EarlyBinder::bind(tcx, bound)
                        .instantiate(tcx, args)
                        .skip_normalization()
                })
                .collect();
            let forwarded = caller_locations.contains(&method) == caller_locations.contains(&target)
                && bounds(method) == theirs
                && js_given.copies(method).next().is_none();
            if forwarded {
                nullable_alike(tcx, method, target);
            }
            forwarded.then_some((method, target))
        })
        .partition(|&(method, _)| {
            super::super::classes::member(tcx, method) == Some(super::super::classes::Member::Namespace)
        });
    (namespaces, prototypes)
}

/// Of `module`'s namespace methods that are functions (`forwarded`, #566,
/// rd292b), where each is written and what's printed of it, as
/// `js::Module::taken_into` says them: its own module's function's JS, of
/// `lowered`'s items, or the re-export of another's, that holds the
/// method's name where it's written. The method has no JS of its own,
/// so its comments, above it, its doc and in its body, are before that
/// (rd758).
pub(super) fn methods_taken_into(
    tcx: TyCtxt<'_>,
    (module, lowered): (LocalModId, &crate::program::LoweredModule),
    (forwarded, fns): (&HashMap<DefId, DefId>, &HashMap<DefId, super::super::FnInfo>),
    sources: &super::super::sources::CapturedSources,
) -> Vec<(crate::js::Span, crate::js::Span)> {
    let mut taken: Vec<_> = (forwarded.iter())
        .filter(|&(method, _)| fns.get(method).is_some_and(|f| f.module == module))
        .filter_map(|(&method, target)| {
            let written = sources.span(tcx.hir_span_with_body(tcx.local_def_id_to_hir_id(method.as_local()?)));
            let (mine, theirs) = (&fns[&method], fns.get(target)?);
            let printed = match theirs.module == module {
                true => lowered.items.iter().find_map(|item| match item {
                    crate::js::Item::Function(f) if f.name == theirs.name => Some(f.span),
                    _ => None,
                })?,
                false => {
                    let holds = |r: &&crate::program::Reexport| r.span.lo <= written.lo && written.hi <= r.span.hi;
                    let named = |r: &&crate::program::Reexport| r.named.iter().any(|(_, a)| a == mine.exported());
                    lowered.reexports.iter().find(holds).filter(named)?.span
                }
            };
            Some((written, printed))
        })
        .collect();
    taken.sort_by_key(|&(method, _)| method.lo);
    taken
}

/// Of `prototypes`, each prototype's method that's the function it passes
/// its parameters on to, and that function (`forwarded`): those whose
/// prototype's property is the function (rd715), one reading no
/// `arguments`, which JS gives all it's given, nor re-exported in its
/// place, `forwarders` (rd575), so a declaration the module's load reads.
pub(super) fn forwarding(
    prototypes: HashMap<DefId, DefId>,
    reads_arguments: &HashSet<DefId>,
    forwarders: &HashMap<DefId, DefId>,
) -> HashSet<DefId> {
    (prototypes.into_iter())
        .filter(|(_, target)| !reads_arguments.contains(target) && !forwarders.contains_key(target))
        .map(|(method, _)| method)
        .collect()
}

/// Whether `method`, the function `target` it passes its parameters on to
/// (`forwarded`), says each of its parameters past its `self` nullable,
/// `#[rust_js::nullable(config)]`, as `target` says its own (#596): a call
/// of it, through a namespace's value too, is the function's, whose `None`
/// is `null` there, as the method's is only where it says so. Else an
/// error naming what to write.
fn nullable_alike(tcx: TyCtxt<'_>, method: DefId, target: DefId) -> bool {
    let nullable = |id: DefId, from: usize| -> Vec<usize> {
        let names = super::super::bindings::nullable_params(tcx, id);
        (tcx.fn_arg_idents(id).iter().enumerate().skip(from))
            .filter(|(_, ident)| ident.is_some_and(|ident| names.contains(&ident.name)))
            .map(|(i, _)| i - from)
            .collect()
    };
    if nullable(method, 1) == nullable(target, 0) {
        return true;
    }
    let names = |id: DefId| {
        let names = super::super::bindings::nullable_params(tcx, id);
        match names.is_empty() {
            true => "none".to_string(),
            false => names
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", "),
        }
    };
    let message = format!(
        "rust-js: `{}` is `{}`, which it passes its parameters on to, so it says the same of them \
         `#[rust_js::nullable(..)]`, as a call of it is that function's (the function's: {}; the method's: {})",
        tcx.def_path_str(method),
        tcx.def_path_str(target),
        names(target),
        names(method),
    );
    tcx.dcx().span_err(tcx.def_span(method), message);
    false
}
