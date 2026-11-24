//! `ParamForm`'s shadow of how generic code holds an iterator, Campaign
//! 6's step 0 (#2539, ADR 1203): of each answer of `given_as_it_is` and
//! `projection_as_it_is`, the bounds of the type that read its value
//! (`reads_its_value`), which hold it as it is wherever they are (ADR
//! 0474). Where one meets a JS iterator, the answers' rules of "another
//! bound reads it" have drifted apart, as #2540's and #2541's did, which
//! `ParamForm`'s one `held_as_iterator` would end. No output changes.

use crate::recognition::{StdItem, std_item, trait_method};
use crate::session::ShadowKind;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, Ty};

/// A `param-form` shadow line, `iterated`, of `id` holding `held` as it
/// is or not, `as_it_is`, which `which` answered: `drifts` where a bound
/// of `held` reads its value and it's a JS iterator, else `agrees`. Only
/// of what's held as an iterator: an `IntoIterator`'s `IntoIter`, or what
/// `id` bounds by `Iterator`, a trait that extends it, or `IntoIterator`.
pub(super) fn shadow_held<'tcx>(tcx: ty::TyCtxt<'tcx>, which: &str, id: DefId, held: Ty<'tcx>, as_it_is: bool) {
    if !crate::session::shadowing(tcx, ShadowKind::ParamForm) {
        return;
    }
    let clauses: Vec<_> = (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .map(|p| p.skip_binder())
        .filter(|p| p.self_ty() == held)
        .collect();
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    let into_iter = matches!(*held.kind(),
        ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if def_id == trait_method(tcx, into_iterator, "IntoIter"));
    let iterated = into_iter
        || clauses.iter().any(|p| {
            p.def_id() == into_iterator
                || p.def_id() == std_item(tcx, StdItem::Iterator)
                || crate::recognition::implies_iterator(tcx, p.def_id())
        });
    if !iterated {
        return;
    }
    let readers: Vec<String> = (clauses.iter())
        .filter(|p| p.def_id() != into_iterator && super::reads_its_value(tcx, p.def_id()))
        .map(|p| tcx.def_path_str(p.def_id()))
        .collect();
    let verdict = if !as_it_is && !readers.is_empty() {
        "drifts"
    } else {
        "agrees"
    };
    let line = format!(
        "iterated\t{which}\t{verdict}\t{}\t{}\t{}\t{}",
        if as_it_is { "as-it-is" } else { "js-iterator" },
        if readers.is_empty() {
            "-".to_string()
        } else {
            readers.join(",")
        },
        tcx.def_path_str(id),
        tcx.erase_and_anonymize_regions(held),
    );
    crate::session::shadow(tcx, ShadowKind::ParamForm, || line);
}
