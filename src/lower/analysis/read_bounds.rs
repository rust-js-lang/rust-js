//! Which bounds of the crate's generic functions a body reads the
//! dictionary of (#1915, ADR 1164): one nothing reads is given none, so
//! `create_details<G, C: Cancelable>` is called with its three arguments,
//! upstream's. Step 1 of #1915: the bounds of traits Copy's, Clone's and
//! the iterators' own analyses don't answer for (`answered_elsewhere`).
//!
//! A bound `P: Tr` of a function is read where the function's MIR, or a
//! closure's or an inline constant's it makes, does any of these:
//! - names a function of `Tr` or of a trait `Tr` extends, a method, a
//!   default method, an associated function without `self`, `P::new()`,
//!   of a `Self` that is or holds `P`;
//! - names an associated constant or type of one, `<P as Tr>::N`;
//! - gives a callee a type that is or holds `P`, where that callee is
//!   given a dictionary of `Tr` or of a trait `Tr` extends for it: the
//!   crate's own as this analysis says, a library's as its contract does;
//! - makes a `dyn` of a value whose type holds `P`;
//! - drops a value of a type built of `P`, whose `Drop` impl may ask any.
//!
//! A type built of `P`, `Vec<P>`, given where a bound asks of it reads each
//! of `P`'s bounds, as an impl of it may ask any. A body with no MIR reads
//! every bound of its function.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::{Body, FnInfo};
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, Operand, Rvalue, visit::Visitor};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

/// A function's type parameter's bound, by the function, the parameter's
/// index and the trait.
pub(in crate::lower) type Bound = (DefId, u32, DefId);

/// Whether the trait `tr`'s bounds are answered by an analysis of their
/// own: `Copy`'s (ADR 0289), `Clone`'s (ADR 0914), the iterators' (ADR
/// 0411) and `IntoIterator`'s; and `Eq` and `Index`, which `bound_of`
/// takes as another trait (#1915, the old owners still to migrate); and a
/// trait that extends one, `trait Copied: Copy`, whose dictionary carries
/// its owner's, `TCopied.Copy()`.
pub(in crate::lower) fn answered_elsewhere(tcx: TyCtxt<'_>, tr: DefId) -> bool {
    use crate::lower::recognition::{StdItem, is_iterator_extension, is_std_def};
    use rustc_hir::attrs::lang_items::LangItem;
    rustc_middle::ty::elaborate::supertrait_def_ids(tcx, tr).any(|tr| {
        [LangItem::Copy, LangItem::Clone, LangItem::Index]
            .into_iter()
            .any(|item| tcx.is_lang_item(tr, item))
            || [StdItem::Iterator, StdItem::IntoIterator, StdItem::Eq]
                .into_iter()
                .any(|item| is_std_def(tcx, tr, item))
            || is_iterator_extension(tcx, tr)
    })
}

/// Whether `id`'s bound `tr` is given no dictionary, as nothing reads it:
/// the crate's own, as this analysis found, `unread`; a library's, as its
/// contract lists them (#1915).
pub(in crate::lower) fn given_none<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    unread: &IndexSet<Bound>,
    id: DefId,
    tr: ty::TraitRef<'tcx>,
) -> bool {
    let ty::Param(param) = tr.self_ty().kind() else {
        return false;
    };
    !answered_elsewhere(tcx, tr.def_id)
        && (unread.contains(&(id, param.index, tr.def_id))
            || !id.is_local() && !foreign.reads(id, param.index, tr.def_id))
}

/// Whether this analysis answers for `id`'s bounds: a function of the
/// crate's, free or an inherent impl's, not a trait's or a trait impl's,
/// whose bounds a dictionary's maker gives.
fn answers_for(tcx: TyCtxt<'_>, id: DefId) -> bool {
    id.is_local()
        && match tcx.def_kind(id) {
            DefKind::Fn => true,
            DefKind::AssocFn => tcx.trait_of_assoc(id).is_none() && tcx.trait_impl_of_assoc(id).is_none(),
            _ => false,
        }
}

/// Each bound of each function `fns` lists that this analysis answers for
/// and whose dictionary no body reads.
pub(in crate::lower) fn unread_bounds<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &IndexMap<DefId, FnInfo>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    compared: &IndexSet<(DefId, u32)>,
) -> IndexSet<Bound> {
    // Each function's own bounds of its type parameters, of traits that
    // give a dictionary: what may go unread.
    let mut candidates: IndexSet<Bound> = IndexSet::default();
    for &id in fns.keys().filter(|&&id| answers_for(tcx, id)) {
        for (clause, _) in tcx.clauses_of(id).instantiate_identity(tcx) {
            let Some(bound) = clause.skip_normalization().as_trait_clause() else {
                continue;
            };
            let tr = bound.skip_binder().trait_ref;
            let ty::Param(param) = tr.self_ty().kind() else {
                continue;
            };
            if answered_elsewhere(tcx, tr.def_id)
                || !crate::lower::traits::has_dictionary_of(tcx, foreign, tr.def_id, tr.self_ty())
                || crate::lower::traits::is_marker(tcx, foreign, tr.def_id)
            {
                continue;
            }
            candidates.insert((id, param.index, tr.def_id));
        }
    }
    let mut read: IndexSet<Bound> = IndexSet::default();
    // Each function's bodies, its closures' and inline constants' too.
    let mut bodies: IndexMap<DefId, Vec<&Body<'tcx>>> = IndexMap::default();
    for &body in all_bodies {
        bodies
            .entry(tcx.typeck_root_def_id(body.def_id))
            .or_default()
            .push(body);
    }
    loop {
        let before = read.len();
        for (&root, of) in &bodies {
            if !candidates.iter().any(|&(id, _, _)| id == root) {
                continue;
            }
            for body in of {
                let Some(mir) = body.mir.as_ref() else {
                    read.extend(candidates.iter().filter(|&&(id, _, _)| id == root).copied());
                    continue;
                };
                let mut reads = Reads {
                    tcx,
                    root,
                    foreign,
                    compared,
                    candidates: &candidates,
                    read: &read,
                    found: Vec::new(),
                    body: &mir.body,
                };
                reads.visit_body(&mir.body);
                for local in mir.body.local_decls.iter() {
                    reads.projections_of(local.ty);
                }
                let found = std::mem::take(&mut reads.found);
                read.extend(found);
            }
        }
        if read.len() == before {
            break;
        }
    }
    candidates.into_iter().filter(|bound| !read.contains(bound)).collect()
}

struct Reads<'a, 'b, 'tcx> {
    tcx: TyCtxt<'tcx>,
    root: DefId,
    foreign: &'a crate::lower::library::Foreign<'b, 'tcx>,
    compared: &'a IndexSet<(DefId, u32)>,
    candidates: &'a IndexSet<Bound>,
    read: &'a IndexSet<Bound>,
    found: Vec<Bound>,
    body: &'a mir::Body<'tcx>,
}

impl<'tcx> Reads<'_, '_, 'tcx> {
    /// The root's bounds of the parameter `index`, each with its trait.
    fn of_param(&self, index: u32) -> impl Iterator<Item = Bound> + '_ {
        let root = self.root;
        (self.candidates.iter())
            .filter(move |&&(id, param, _)| id == root && param == index)
            .copied()
    }

    /// The root's type parameters `ty` is or holds.
    fn params_in(&self, ty: ty::Ty<'tcx>) -> Vec<u32> {
        (ty.walk().filter_map(|part| part.as_type()))
            .filter_map(|part| match part.kind() {
                ty::Param(param) => Some(param.index),
                _ => None,
            })
            .collect()
    }

    /// Reads `P: Tr` of each of the root's bounds of a parameter `ty` is
    /// or holds whose dictionary carries `asked`'s: `Tr` is `asked` or
    /// extends it. Where none carries it, an impl answers, a blanket one,
    /// `impl<T: Display> ToString for T`, which may ask any of `P`'s; and a
    /// type built of `P` may ask any too: each of `P`'s bounds. Whether
    /// one of `P`'s dictionaries carried it, which is then the impl.
    fn asked(&mut self, ty: ty::Ty<'tcx>, asked: DefId) -> bool {
        for index in self.params_in(ty) {
            let reached: Vec<Bound> = self.of_param(index).collect();
            let carries = |bound: &Bound| {
                rustc_middle::ty::elaborate::supertrait_def_ids(self.tcx, bound.2).any(|tr| tr == asked)
            };
            // Carried by any of `P`'s bounds, one answered elsewhere too,
            // `T: Eq` of `a == b`.
            let carried = self.carried(ty, asked);
            for bound in &reached {
                if !carried || carries(bound) {
                    self.found.push(*bound);
                }
            }
            if carried {
                return true;
            }
        }
        false
    }

    /// Reads what a use of `tr` of `own` and the trait's other arguments,
    /// `args`, asks: `own`'s dictionary of it, where one of a parameter's
    /// carries it, the impl, which the other arguments don't select, as
    /// `a == b` of `T: Ord` gives `T` for `Rhs`; else an impl they select,
    /// `u32: Into<P>`'s `impl<T, U: From<T>> Into<U> for T`, which may ask
    /// any of their bounds.
    fn trait_use(&mut self, own: ty::Ty<'tcx>, tr: DefId, args: impl Iterator<Item = ty::Ty<'tcx>>) {
        if self.asked(own, tr) {
            return;
        }
        for ty in args {
            self.all_of(ty);
        }
    }

    /// Reads each of the root's bounds of each parameter `ty` holds.
    fn all_of(&mut self, ty: ty::Ty<'tcx>) {
        for index in self.params_in(ty) {
            let all: Vec<Bound> = self.of_param(index).collect();
            self.found.extend(all);
        }
    }

    /// Each of `ty`'s projections of a parameter's associated type,
    /// `<P as Tr>::Item`, a read of `Tr`, whose dictionary carries its
    /// facts: not one behind a reference or a pointer, whose JS is the
    /// value itself, which asks none of it, as `&'static C::Details<G>` of
    /// createDetails' result doesn't.
    fn projections_of(&mut self, ty: ty::Ty<'tcx>) {
        let mut walk = ty.walk();
        while let Some(part) = walk.next() {
            let Some(part) = part.as_type() else { continue };
            match part.kind() {
                ty::Ref(..) => walk.skip_current_subtree(),
                _ if crate::lower::pointers::raw_pointee(self.tcx, part).is_some() => walk.skip_current_subtree(),
                ty::Alias(
                    _,
                    alias @ ty::AliasTy {
                        kind: ty::Projection { def_id },
                        ..
                    },
                ) => {
                    let tr = self.tcx.parent(*def_id);
                    self.asked(alias.self_ty(), tr);
                }
                _ => {}
            }
        }
    }

    /// A function named, called or made a value, `callee` of `args`.
    fn named(&mut self, callee: DefId, args: ty::GenericArgsRef<'tcx>) {
        let tcx = self.tcx;
        // What it's given a type of, `size_of::<C::Details>()`, whose facts
        // a projection's dictionary carries.
        for ty in args.types() {
            self.projections_of(ty);
        }
        // A comparison of a parameter every caller gives a JS number, JS's
        // own operator, which reads no dictionary (#1874).
        if self.compares_by_operator(callee, args) {
            return;
        }
        // Of a trait, of a `Self` that is or holds a parameter: where no
        // dictionary of a parameter's carries it, the impl rustc selects,
        // `impl<A: PartialEq<B>, B> PartialEq<&B> for &A` of `&T == &T`,
        // asks what its bounds do.
        if let Some(tr) = tcx.trait_of_assoc(callee) {
            let mut types = args.types();
            if let Some(own) = types.next() {
                match self.selected_impl(own, tr, callee, args) {
                    // Its bounds are the trait method's, `Self: Tr` the
                    // impl itself.
                    Some(instance) => return self.bounds_given(instance.def_id(), instance.args),
                    None => {
                        let trait_args = tcx.generics_of(tr).count();
                        self.trait_use(own, tr, types.take(trait_args.saturating_sub(1)));
                    }
                }
            }
        }
        // A library's function that isn't a binding, given a parameter, may
        // ask any of its bounds where no clause says which, std's
        // `TypeId::of::<T>()` of its `Any`.
        if !callee.is_local()
            && !crate::lower::bindings::is_binding(tcx, callee)
            && tcx.trait_of_assoc(callee).is_none()
        {
            for ty in args.types() {
                self.all_of(ty);
            }
        }
        self.bounds_given(callee, args);
    }

    /// Whether `callee` of `args` is `PartialOrd`'s `lt`, `le`, `gt` or `ge`,
    /// or `PartialEq`'s `eq` or `ne`, of two of a type parameter of the root's
    /// that's compared by JS's own operators (`compares_by_operator_param`),
    /// as the lowering writes them (#1874).
    fn compares_by_operator(&self, callee: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        use rustc_hir::attrs::lang_items::LangItem;
        let tcx = self.tcx;
        let operators: &[&str] = match tcx.trait_of_assoc(callee) {
            Some(tr) if tcx.is_lang_item(tr, LangItem::PartialOrd) => &["lt", "le", "gt", "ge"],
            Some(tr) if tcx.is_lang_item(tr, LangItem::PartialEq) => &["eq", "ne"],
            _ => return false,
        };
        let mut types = args.types();
        let (Some(own), Some(other)) = (types.next(), types.next()) else {
            return false;
        };
        let Some(index) = crate::lower::generic_facts::type_parameter_of(own) else {
            return false;
        };
        own == other
            && operators.contains(&tcx.item_name(callee).as_str())
            && crate::lower::analysis::compares_by_operator_param(tcx, self.foreign, self.compared, (self.root, index))
    }

    /// Whether a dictionary of one of the parameter `own` is's bounds
    /// carries `tr`, which is then the impl.
    fn carried(&self, own: ty::Ty<'tcx>, tr: DefId) -> bool {
        let ty::Param(own) = own.kind() else { return false };
        (self
            .tcx
            .clauses_of(self.root)
            .instantiate_identity(self.tcx)
            .into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .map(|bound| bound.skip_binder().trait_ref)
        .any(|bound| {
            matches!(bound.self_ty().kind(), ty::Param(param) if param.index == own.index)
                && rustc_middle::ty::elaborate::supertrait_def_ids(self.tcx, bound.def_id).any(|of| of == tr)
        })
    }

    /// The impl's method rustc selects for `callee` of `args`, a method of
    /// `tr` of `own`, where no dictionary of a parameter's carries it.
    fn selected_impl(
        &self,
        own: ty::Ty<'tcx>,
        tr: DefId,
        callee: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<ty::Instance<'tcx>> {
        if self.carried(own, tr) {
            return None;
        }
        let typing_env = ty::TypingEnv::post_analysis(self.tcx, self.root);
        let instance = crate::lower::items::resolve_instance_in(self.tcx, typing_env, callee, args).ok()??;
        match instance.def {
            ty::InstanceKind::Item(def) if self.tcx.trait_impl_of_assoc(def).is_some() => Some(instance),
            _ => None,
        }
    }

    /// Reads what `callee`'s own bounds ask of what it's given, `args`,
    /// each one given a dictionary.
    fn bounds_given(&mut self, callee: DefId, args: ty::GenericArgsRef<'tcx>) {
        let tcx = self.tcx;
        // As declared, of a parameter of its own, and as given.
        let declared = tcx.clauses_of(callee).instantiate_identity(tcx);
        let given = tcx.clauses_of(callee).instantiate(tcx, args);
        for ((declared, _), (clause, _)) in declared.into_iter().zip(given) {
            let Some(bound) = clause.skip_normalization().as_trait_clause() else {
                continue;
            };
            let tr = bound.skip_binder().trait_ref;
            if !crate::lower::traits::has_dictionary_of(tcx, self.foreign, tr.def_id, tr.self_ty())
                || crate::lower::traits::is_marker(tcx, self.foreign, tr.def_id)
            {
                continue;
            }
            let of = (declared.skip_normalization().as_trait_clause())
                .map(|declared| *declared.skip_binder().trait_ref.self_ty().kind());
            let given = match of {
                // The crate's, as this analysis says; a library's, as its
                // contract does; any other, given one.
                Some(ty::Param(param)) if answers_for(tcx, callee) => {
                    let bound = (callee, param.index, tr.def_id);
                    !self.candidates.contains(&bound) || self.read.contains(&bound)
                }
                Some(ty::Param(param)) if !callee.is_local() => self.foreign.reads(callee, param.index, tr.def_id),
                _ => true,
            };
            if given {
                self.trait_use(tr.self_ty(), tr.def_id, tr.args.types().skip(1));
            }
        }
    }
}

impl<'tcx> Visitor<'tcx> for Reads<'_, '_, 'tcx> {
    fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, location: mir::Location) {
        if let Some((callee, args)) = crate::lower::fn_def(constant.ty()) {
            self.named(callee, args);
        }
        // An associated constant of a parameter's trait, `<P as Tr>::N`; or
        // another constant of what holds a parameter, an inline `const {
        // T::V }`, a body of its own, which may ask any of its bounds.
        if let mir::Const::Unevaluated(unevaluated, _) = constant.const_ {
            match self.tcx.trait_of_assoc(unevaluated.def) {
                Some(tr) if let Some(own) = unevaluated.args.types().next() => {
                    self.asked(own, tr);
                }
                _ => {
                    for ty in unevaluated.args.types() {
                        self.all_of(ty);
                    }
                }
            }
        }
        self.super_const_operand(constant, location);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, location: mir::Location) {
        // A `dyn` of what holds a parameter.
        if let Rvalue::Cast(mir::CastKind::PointerCoercion(ty::adjustment::PointerCoercion::Unsize, _), operand, target) =
            rvalue
            && target
                .walk()
                .filter_map(|part| part.as_type())
                .any(|part| matches!(part.kind(), ty::Dynamic(..)))
        {
            let source = operand.ty(self.body, self.tcx);
            self.all_of(source);
        }
        if let Rvalue::Aggregate(kind, _) = rvalue
            && let mir::AggregateKind::Adt(_, _, args, _, _) | mir::AggregateKind::Closure(_, args) = **kind
        {
            for ty in args.types() {
                self.projections_of(ty);
            }
        }
        self.super_rvalue(rvalue, location);
    }

    fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: mir::Location) {
        // A drop of a type built of a parameter, whose `Drop` impl may ask
        // any of its bounds, `impl<E: Engine> Drop for EncoderWriter<E>`;
        // a parameter's own is its `dropT`, which no bound gives.
        if let mir::TerminatorKind::Drop { place, .. } = &terminator.kind {
            let ty = place.ty(self.body, self.tcx).ty;
            if !matches!(ty.kind(), ty::Param(_)) {
                self.all_of(ty);
            }
        }
        self.super_terminator(terminator, location);
    }

    fn visit_operand(&mut self, operand: &Operand<'tcx>, location: mir::Location) {
        self.super_operand(operand, location);
    }
}
