//! `CellForm`'s bridge from a body being lowered (#2490, ADR 1199): the
//! crate's `CellForms`, proved once, each body's `Proving` of its own
//! typing and the drops it's given, and the shadow beside the form a body
//! takes. Split from ref_cells.rs by concept.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `CellForm`'s shadow of `body`, the MIR being lowered, where the
    /// session records it (#2490, ADR 1199): each borrow's proof
    /// (`analysis::shadow_cell_forms_of`) beside the form its lowering takes,
    /// of `QuietCells`' blocks, those unchecked, `quiet`, and those checked
    /// and not counted, `momentary`; else as `std_types/cell.rs` asks, a type
    /// no borrow of which is counted unchecked; a flattened state as its
    /// declaration says. The crate's fields used whole are found once.
    pub fn shadow_cell_forms(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        (quiet, momentary): (
            &IndexSet<rustc_middle::mir::BasicBlock>,
            &IndexSet<rustc_middle::mir::BasicBlock>,
        ),
    ) {
        use crate::analysis::Today;
        use crate::session::ShadowKind;
        if !crate::session::shadowing(self.tcx, ShadowKind::CellForm) {
            return;
        }
        let today = |block, cell, field: Option<&'tcx ty::FieldDef>| match () {
            _ if field.is_some_and(|field| crate::bindings::is_flattened_state_field(self.tcx, field)) => {
                Today::Declared
            }
            _ if quiet.contains(&block) || self.quiet_ref_cell(cell, false) => Today::Unchecked,
            _ if momentary.contains(&block) => Today::Momentary,
            _ => Today::Counted,
        };
        let thir = (body.source.def_id() == self.frame.owner).then_some(self.frame.thir);
        let caches = crate::session::caches(self.tcx);
        let (wholes, proofs) = self.cell_form_proofs(&caches);
        for (&field, whole) in wholes {
            crate::session::shadow(self.tcx, ShadowKind::CellForm, || {
                let lent = if whole.lent_to_borrows {
                    "lent-to-borrows"
                } else {
                    "used"
                };
                format!("whole\t{}\t{lent}\t{}", self.tcx.def_path_str(field), whole.why)
            });
        }
        self.proving_of(body.source.def_id(), &mut |proving| {
            crate::analysis::shadow_cell_forms_of(proving, body, (thir, wholes, proofs), &today);
        });
    }

    /// Each `RefCell` borrow's form, `CellForm`'s proof's of the crate's
    /// bodies, found once (#2490's flip): what THIR's facts ask of each
    /// borrow, whether it's counted (`find_facts`).
    pub(crate) fn cell_forms(&self) -> Rc<crate::analysis::CellForms> {
        let caches = crate::session::caches(self.tcx);
        if let Some(forms) = caches.cell_forms.get() {
            return forms.clone();
        }
        let (wholes, proofs) = self.cell_form_proofs(&caches);
        let forms = Rc::new(crate::analysis::CellForms::of_crate(
            self.local_mir_bodies(),
            &|owner, each| self.proving_of(owner, each),
            (wholes, proofs),
        ));
        caches.cell_forms.get_or_init(|| forms).clone()
    }

    /// The crate's own bodies with MIR, its functions' and its closures',
    /// each with its THIR.
    fn local_mir_bodies(
        &self,
    ) -> impl Iterator<Item = (&'a rustc_middle::mir::Body<'tcx>, &'a rustc_middle::thir::Thir<'tcx>)> {
        (self.krate.bodies.values().chain(self.krate.closures.values()))
            .filter(|body| body.def_id.is_local())
            .filter_map(|body| Some((&body.mir.as_ref()?.body, &body.thir)))
    }

    /// The drops `root`'s body is given, each type parameter's its
    /// lowering takes a drop of (`drops_taken`, ADR 0098): what a value of
    /// one drops, as its lowering runs it, nothing where it's given none
    /// (#1321). The one answer `CellForm`'s proof and `find_facts` ask.
    pub(super) fn drops_given(&self, root: DefId) -> TypeDrops<'tcx> {
        let mut state = TypeDrops::default();
        for index in self.drops_taken(root) {
            state.give(crate::traits::drop_slot(self.tcx, self.slots(), root, index));
        }
        state
    }

    /// `owner`'s body's `Proving`, of its own typing, given to `each`.
    fn proving_of(&self, owner: DefId, each: &mut dyn FnMut(&crate::analysis::Proving<'_, '_, 'tcx>)) {
        // Its drops asked as the lowering's are, of the body being lowered.
        let lowering = self.recognition();
        let recognition = crate::recognition::Recognition {
            typing_env: ty::TypingEnv::post_analysis(self.tcx, self.tcx.typeck_root_def_id(owner)),
            ..lowering
        };
        let drops = |ty| {
            super::drop_runs(
                lowering,
                (self.krate.library, self.krate.closure_facts),
                self.krate.counted,
                ty,
            )
        };
        // A type parameter's value's drop, as its lowering runs it: what
        // the drop its function is given runs, nothing where it's given
        // none (`drops_taken`, ADR 0098), as `find_ref_cells` asks it.
        let state = self.drops_given(self.tcx.typeck_root_def_id(owner));
        let query = without_dictionaries(
            recognition,
            (false, self.krate.closure_facts),
            self.krate.counted,
            &state,
        );
        let given = |ty| query.drop_may_ask(ty);
        each(&crate::analysis::Proving {
            recognition: &recognition,
            borrows: self.krate.cell_borrows,
            drops: &drops,
            given_drops: &given,
        });
    }

    /// The crate's fields used whole and its types' proofs, `CellForm`'s,
    /// found once of its bodies.
    fn cell_form_proofs<'c>(
        &self,
        caches: &'c crate::session::Caches,
    ) -> (
        &'c IndexMap<DefId, crate::analysis::Whole>,
        &'c crate::analysis::TypeProofs,
    ) {
        let wholes = caches.cell_form_wholes.get_or_init(|| {
            crate::analysis::whole_uses(&self.recognition(), self.local_mir_bodies().map(|(mir, _)| mir))
        });
        let proofs = caches.cell_form_types.get_or_init(|| {
            crate::analysis::TypeProofs::of_crate(
                self.tcx,
                self.local_mir_bodies(),
                &|owner, each| self.proving_of(owner, each),
                wholes,
            )
        });
        (wholes, proofs)
    }
}
