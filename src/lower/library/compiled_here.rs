//! Which of its libraries' items the crate compiles itself, from the MIR
//! their metadata keeps, and why (ADR 0914): one its library's JS doesn't
//! export, a writer given more than its library's JS takes (#1026), one
//! whose JS may merge what the crate compares (#1052), and one whose library
//! keeps no `Vec`'s capacity, which the crate reads (#1180's (5)).

use super::{Foreign, key};
use crate::collections::IndexSet;
use rustc_hir::def::DefKind;
use rustc_span::def_id::DefId;

/// A library's functions whose JS may give one function where Rust makes
/// two: those its `Rc`s merge (`Counted::merges`), those whose lowering
/// made a closure the function it passes on to as the crate reads no
/// identity (`Dependencies::merges_identities`), and each of its functions
/// that names one, by what each lowered function names (`uses`): its JS
/// calls reach them, so a consumer that compares a function's identity
/// compiles its own of each (#1052).
pub(in crate::lower) fn merging<'f>(
    rcs: &IndexSet<DefId>,
    lowered: impl Iterator<Item = (DefId, &'f crate::lower::LoweredFn)> + Clone,
) -> IndexSet<DefId> {
    let mut merging: IndexSet<DefId> = rcs.iter().copied().collect();
    merging.extend(
        (lowered.clone())
            .filter(|(_, item)| item.dependencies.merges_identities)
            .map(|(id, _)| id),
    );
    let uses: Vec<(DefId, DefId)> = lowered
        .flat_map(|(_, item)| item.dependencies.uses.iter().copied())
        .collect();
    loop {
        let more: Vec<DefId> = (uses.iter())
            .filter(|(from, to)| merging.contains(to) && !merging.contains(from))
            .map(|&(from, _)| from)
            .collect();
        if more.is_empty() {
            return merging;
        }
        merging.extend(more);
    }
}

impl<'a, 'tcx> Foreign<'a, 'tcx> {
    /// Of `id`, a library's writer, what the crate's writers take that its
    /// library's don't, which it's compiled here to take (ADR 0914,
    /// amended; #1026): a placeholder's options, `{:>4}` or `{:#?}`, or an
    /// integer's `{:?}` in hex, `{:x?}`. `None` of another, or of one whose
    /// library's take all the crate's do. A writer: a formatting trait's
    /// impl one of whose methods reads its `Formatter`'s options, and a
    /// function that does, as its library's manifest lists them.
    pub(in crate::lower) fn writes_more(&self, id: DefId) -> Option<&'static str> {
        let tcx = self.tcx;
        if id.is_local() || !self.is_library(id.krate) {
            return None;
        }
        // Not `@rust-js/std`'s, compiled once for every program (ADR 0561).
        let library = self.library(id.krate).filter(|library| !library.builtin)?;
        let more = match () {
            _ if self.writes.options && !library.writers_take_options => "a placeholder's options, `{:>4}` or `{:#?}`",
            _ if self.writes.hex && !library.debug_hex => "an integer's `{:?}` in hex, `{:x?}`",
            _ => return None,
        };
        // One its library's compile read as reading a `Formatter`'s
        // options, or an impl one of whose methods does; another, which
        // only writes its text, takes nothing more.
        let reads = |f: DefId| library.reads_formatter_options.contains(&key(tcx, f));
        let root = tcx.typeck_root_def_id(id);
        let writer = match tcx.def_kind(root) {
            DefKind::Impl { of_trait: true } => tcx.associated_item_def_ids(root).iter().any(|&f| reads(f)),
            _ => reads(root),
        };
        writer.then_some(more)
    }

    /// Whether `id`, a library's function or an impl of it, may give the
    /// crate one value where Rust makes two, which the crate may compare:
    /// it compares a function's identity (`reads_fn_identity`), and its
    /// library's compile, which compares none, made an `Rc<dyn Fn>` of a
    /// closure cloned uncounted (ADR 0959) or a closure the function it
    /// passes on to (ADR 0605), as its manifest lists it and each function
    /// that calls one; an impl one of whose methods does. Compiled here,
    /// where the crate's own compile keeps them apart (ADR 0914, amended;
    /// #1052).
    pub(in crate::lower) fn merges_identities(&self, id: DefId) -> bool {
        let tcx = self.tcx;
        if !self.reads_fn_identity || id.is_local() || !self.is_library(id.krate) {
            return false;
        }
        // Not `@rust-js/std`'s, compiled once for every program (ADR 0561).
        let Some(library) = self.library(id.krate).filter(|library| !library.builtin) else {
            return false;
        };
        let merges = |f: DefId| library.merges_identities.contains(&key(tcx, f));
        let root = tcx.typeck_root_def_id(id);
        match tcx.def_kind(root) {
            DefKind::Impl { of_trait: true } => tcx.associated_item_def_ids(root).iter().any(|&f| merges(f)),
            _ => merges(root),
        }
    }

    /// Whether `id`, a library's function or impl, is compiled here to keep
    /// a `Vec`'s capacity (#1180's (5), the user's decision): the crate
    /// keeps capacities, as it or a library it uses reads one (ADR 0419),
    /// and `id`'s library, not `@rust-js/std`, keeps none, so its JS grows
    /// a `Vec` as a JS array does, which no `capacity()` of the crate's can
    /// tell from std's. Its copy here grows each as std does, and so does
    /// each of its library's functions it reaches, compiled here in turn.
    pub(in crate::lower) fn keeps_more(&self, id: DefId) -> bool {
        if id.is_local() || !self.is_library(id.krate) || !(self.asks_capacity || self.keeps_capacity()) {
            return false;
        }
        self.library(id.krate)
            .is_some_and(|library| !library.builtin && !library.keeps_capacity)
    }

    /// Whether a call of `id`, a library's function, given `args`, calls
    /// one the crate compiles itself as its JS may merge what the crate
    /// compares (`merges_identities`): of a trait's method, the impl's
    /// method it resolves to, or, of generic code's, where it resolves to
    /// none yet, and of a `dyn Trait`'s, which resolves to the trait's own
    /// (#1055), each other crate's impl's of the trait, as the crate's own
    /// are its own (#1052).
    pub(super) fn merged_here(&self, id: DefId, args: rustc_middle::ty::GenericArgsRef<'tcx>) -> bool {
        use rustc_middle::ty::TypeVisitableExt;
        if self.merges_identities(id) {
            return true;
        }
        let tcx = self.tcx;
        let Some(of_trait) = tcx.trait_of_assoc(id) else {
            return false;
        };
        let args = tcx.erase_and_anonymize_regions(args);
        let env = rustc_middle::ty::TypingEnv::fully_monomorphized();
        // Not one that resolves to the trait's own method, a `dyn Trait`'s
        // call through its dictionary, which may be any impl's (#1055).
        if !args.has_non_region_param()
            && let Ok(Some(instance)) = rustc_middle::ty::Instance::try_resolve(tcx, env, id, args)
            && instance.def_id() != id
        {
            return self.merges_identities(instance.def_id());
        }
        let impls: Vec<DefId> = tcx.all_impls(of_trait).filter(|imp| !imp.is_local()).collect();
        !impls.is_empty()
            && impls.iter().all(|&imp| {
                (tcx.impl_item_implementor_ids(imp).get(&id)).is_some_and(|&method| self.merges_identities(method))
            })
    }

    /// Is `id` a library's that its JS doesn't export, which this crate
    /// compiles itself where it uses it, from the MIR its library's metadata
    /// keeps (ADR 0914, amended): a trait impl of its, whose dictionary its
    /// program makes only where its own code asks for it, and its methods;
    /// or a trait's default, which a consumer's impl of the trait calls. Not
    /// the method of an impl whose dictionary it exports, called through it
    /// (`through_dictionary`); nor a derive's that rust-js writes itself,
    /// `==` of a struct, nor a binding's, JS's; nor a function or an
    /// inherent method, which its program exports where it's public, as its
    /// manifest says (`unlisted`). And a writer the crate gives more than its
    /// library's JS takes, exported or not (`writes_more`, #1026), and one
    /// whose JS may merge what the crate compares (`merges_identities`,
    /// #1052).
    pub(in crate::lower) fn compiles(&self, id: DefId) -> bool {
        use rustc_hir::def::DefKind;
        let tcx = self.tcx;
        let own = |imp: DefId| self.has_impl(imp) && self.item(imp).is_none();
        let unexported = || {
            self.item(id).is_none()
                && match tcx.def_kind(id) {
                    DefKind::Impl { of_trait: true } => own(id),
                    DefKind::AssocFn if tcx.is_mir_available(id) && !crate::lower::bindings::is_binding(tcx, id) => {
                        match tcx.trait_impl_of_assoc(id) {
                            Some(imp) => own(imp),
                            None => tcx.trait_of_assoc(id).is_some(),
                        }
                    }
                    _ => false,
                }
        };
        // Or a writer given more than its library's JS takes (#1026), one
        // whose JS may merge what the crate compares (#1052), or one whose
        // library keeps no capacity the crate reads (#1180's (5)).
        let writes_more = || {
            (self.writes_more(id).is_some() || self.merges_identities(id) || self.keeps_more(id))
                && (matches!(tcx.def_kind(id), DefKind::Impl { .. }) || tcx.is_mir_available(id))
                && !crate::lower::bindings::is_binding(tcx, id)
        };
        !id.is_local() && self.is_library(id.krate) && !self.is_instance(id) && (unexported() || writes_more())
    }

    /// The impl of `id`, a library's impl's method its JS doesn't export by
    /// name, whose dictionary it does: what a call of it is a call through,
    /// `strToKebabCase.toKebabCase(title)` (ADR 0914, amended).
    pub(in crate::lower) fn through_dictionary(&self, id: DefId) -> Option<DefId> {
        let imp = self.tcx.trait_impl_of_assoc(id)?;
        (!id.is_local() && !self.is_instance(id) && self.item(id).is_none() && self.item(imp).is_some()).then_some(imp)
    }

    /// Whether `id`, a library's, has JS the program calls: one its
    /// manifest exports, or one the crate compiles itself, which it may have
    /// compiled already, an instance (ADR 0914). The one answer, which each
    /// question of whether a library's item can be reached asks, the crate's
    /// own beside it where it asks of those too: one asked of `compiles`
    /// alone, false once it's an instance, took a library's own `size_hint`
    /// for std's after its first call (#1023).
    pub(in crate::lower) fn compiled(&self, id: DefId) -> bool {
        !id.is_local() && (self.item(id).is_some() || self.is_instance(id) || self.compiles(id))
    }
}
