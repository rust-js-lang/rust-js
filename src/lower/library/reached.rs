//! What the libraries can hand the crate, of what its bodies' values reach
//! of theirs (ADR 0626): a library's function's signature, a library's
//! type's fields, and the `Rc`s they hold, counted where their library
//! counts them (ADRs 0844, 0852).

use super::{Foreign, Reached, library_key};
use crate::collections::IndexSet;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> Foreign<'a, 'tcx> {
    /// What the libraries can hand this crate (ADR 0626): of each type its
    /// bodies' values have, what a library's part of it reaches, a
    /// library's function's signature, a library's type's fields. Not what
    /// the crate's own types hold, which it sees.
    pub(in crate::lower) fn reached(&self, all_bodies: &[&crate::lower::Body<'tcx>]) -> Reached<'tcx> {
        let mut reached = Reached::default();
        if !self.any() {
            return reached;
        }
        let mut seen = IndexSet::default();
        for body in all_bodies {
            for expr in body.thir.exprs.iter() {
                for part in expr.ty.walk().filter_map(|part| part.as_type()) {
                    self.reach(part, None, &mut seen, &mut reached);
                }
            }
        }
        reached
    }

    /// What `ty` reaches of a library's, `exported` of the library whose
    /// export it's in.
    fn reach(
        &self,
        ty: rustc_middle::ty::Ty<'tcx>,
        exported: Option<rustc_span::def_id::CrateNum>,
        seen: &mut IndexSet<(rustc_middle::ty::Ty<'tcx>, Option<rustc_span::def_id::CrateNum>)>,
        reached: &mut Reached<'tcx>,
    ) {
        use rustc_middle::ty;
        if !seen.insert((ty, exported)) {
            return;
        }
        let tcx = self.tcx;
        let library = |id: DefId| self.is_library(id.krate);
        // Not what a closure of the crate's holds, which a library's generic
        // function is given, `then(move |_| ..)`'s: the crate made it of
        // what it sees, nothing the library hands it.
        let mut parts = |of: DefId, tys: Vec<ty::Ty<'tcx>>, reached: &mut Reached<'tcx>| {
            for t in tys {
                let mut walk = t.walk();
                while let Some(part) = walk.next() {
                    let Some(part) = part.as_type() else { continue };
                    if let ty::Closure(id, _) | ty::CoroutineClosure(id, _) | ty::Coroutine(id, _) = *part.kind()
                        && id.is_local()
                    {
                        walk.skip_current_subtree();
                        continue;
                    }
                    self.reach(part, Some(of.krate), seen, reached);
                }
            }
        };
        match *ty.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Opaque { def_id },
                    ..
                },
            ) => reached.functions |= library(def_id),
            ty::Dynamic(predicates, ..)
                if let Some(krate) = exported
                    && crate::lower::recognition::is_fn_dyn(tcx, predicates) =>
            {
                reached.fn_dyns_of.insert(library_key(tcx, krate));
            }
            ty::FnDef(..)
                if let Some((id, args)) = crate::lower::fn_def(ty)
                    && library(id) =>
            {
                let declared = tcx
                    .instantiate_bound_regions_with_erased(tcx.fn_sig(id).instantiate_identity().skip_normalization());
                // Not of one the crate compiles itself as its library's JS
                // may merge what it compares, whose `Rc`s it makes as it
                // counts them (#1052).
                if !self.merged_here(id, args) {
                    self.holds_rcs(id, (declared.inputs_and_output.to_vec(), args), reached);
                }
                let sig = tcx.fn_sig(id).instantiate(tcx, args).skip_normalization().skip_binder();
                parts(id, sig.inputs_and_output.to_vec(), reached);
            }
            ty::Adt(adt, args) if library(adt.did()) => {
                let declared = adt
                    .all_fields()
                    .map(|field| tcx.type_of(field.did).instantiate_identity().skip_normalization());
                self.holds_rcs(adt.did(), (declared.collect(), args), reached);
                let fields = adt.all_fields().map(|field| field.ty(tcx, args).skip_normalization());
                parts(adt.did(), fields.collect(), reached);
            }
            _ => {}
        }
    }
}
