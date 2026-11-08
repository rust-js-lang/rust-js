//! Whether the crate steps a value from its back, by its MIR identity
//! (back_stepped's `stepped_locals`, `made_into`, #814, #851), and whether a
//! value made before already has a back, by its form down to what its type
//! says (#851): the one answer of each, which makers and what asks a value
//! read.

use super::*;

/// A local a call makes its value into or is given, and its type, regions
/// erased (`made_into`, `made_from`, #851).
pub(in crate::lower) type MadeLocal<'tcx> = (rustc_middle::mir::Local, ty::Ty<'tcx>);

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `ty`, a map's or a filter's, is one the crate steps from its
    /// back (ADR 0545), made knowing it where it's made: the one answer, of
    /// what's made and what's stepped.
    /// Whether `ty` is the type of what the call being lowered makes, into a
    /// local back_stepped's `stepped_locals` says the crate steps from its
    /// back, lent or given (ADR 0545, #851): by the value, its maker's MIR
    /// identity, not its type, which another value of it need not share. Of
    /// what's asked of a value made before, never: its mark says, carried by
    /// its local, `has_a_back`'s.
    pub(in crate::lower) fn stepped_from_its_back(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = self.tcx.erase_and_anonymize_regions(self.reveal(ty.peel_refs()));
        if !self.back_by_value(ty) || self.by_set.get() {
            return (self.krate.back_stepped.types).contains(&ty);
        }
        let stepped = self.krate.back_stepped.typed.contains(&ty)
            || !self.asking_made.get()
                && (self.made_into.iter().chain(&self.made_from)).any(|&(local, made)| {
                    let local = (self.krate.back_stepped.borrows)
                        .get(&(self.frame.owner, local))
                        .copied()
                        .unwrap_or(local);
                    made == ty && self.local_stepped_from_its_back(local).is_some()
                });
        // The per-type set, while it's compared (#851's migration): each
        // answer it gives otherwise said on stderr where
        // `RUST_JS_BACK_STEPPED_CROSS_CHECK` is set.
        if std::env::var_os("RUST_JS_BACK_STEPPED_CROSS_CHECK").is_some()
            && stepped != (self.krate.back_stepped.types).contains(&ty)
        {
            let fact = self
                .made_into
                .map(|(local, made)| (made == ty, self.local_stepped_from_its_back(local)));
            eprintln!(
                "back-stepped cross-check\t{}\t{ty}\tvalue={stepped}\t{fact:?}",
                self.tcx.def_path_str(self.item)
            );
        }
        stepped
    }

    /// Whether whether a `ty` is made knowing its back is its value's fact,
    /// its maker's MIR identity, not back_stepped's per-type set: a map's or
    /// a filter's (#851 phase B); another kind's by the set still. The one
    /// answer of what's asked and what's marked.
    pub(in crate::lower) fn back_by_value(&self, ty: ty::Ty<'tcx>) -> bool {
        crate::lower::recognition::mapped_source(self.tcx, ty).is_some()
            || crate::lower::recognition::filtered_source(self.tcx, ty).is_some()
            || crate::lower::recognition::chained_sources(self.tcx, ty).is_some()
            || crate::lower::recognition::copied_source(self.tcx, ty).is_some()
            || crate::lower::recognition::cloned_source(self.tcx, ty).is_some()
            || crate::lower::recognition::zipped_sources(self.tcx, ty).is_some()
    }

    /// Whether `local`, of the body being lowered, holds what the crate
    /// steps from its back (back_stepped's `stepped_locals`, #814): by the
    /// MIR identity of what holds a value, not its type, which another value
    /// of it need not share. The one answer of a counted adapter's, a
    /// `skip(n)`'s, a `take(n)`'s or a `step_by(n)`'s.
    pub(in crate::lower) fn local_stepped_from_its_back(
        &self,
        local: rustc_middle::mir::Local,
    ) -> Option<crate::lower::analysis::SteppedLocal> {
        (self.krate.back_stepped.locals)
            .get(&(self.frame.owner, local))
            .copied()
    }

    /// Whether what the std iterator method being lowered makes, into
    /// `made_into`, is given to what steps it from its back, made knowing it
    /// by its maker (`SteppedLocal::Given`).
    /// `make`, a call's lowering, told where it makes its value, `into`, its
    /// maker's MIR identity, which `made_stepped_from_its_back` asks (#814).
    pub(in crate::lower) fn making_into<T>(
        &mut self,
        (into, of): (Option<MadeLocal<'tcx>>, Vec<MadeLocal<'tcx>>),
        make: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let outer = std::mem::replace(&mut self.made_into, into);
        let outer_from = std::mem::replace(&mut self.made_from, of);
        let made = make(self);
        self.made_into = outer;
        self.made_from = outer_from;
        made
    }

    pub(in crate::lower) fn made_stepped_from_its_back(&self) -> bool {
        self.made_into
            .and_then(|(local, _)| self.local_stepped_from_its_back(local))
            == Some(crate::lower::analysis::SteppedLocal::Given)
    }

    /// `value`, a double-ended iterator of type `ty`, as a JS iterator that
    /// knows its back (ADR 0429): an array's, its `$iter`; a range's,
    /// `$iterator(range, $rangeNext, false, $rangeNextBack)`; the crate's
    /// own, stepped by its impls' `next` and `next_back`; one already, a
    /// `dyn`'s or a type parameter's, itself.
    /// Whether `value`, of type `ty`, has a back by what its form holds,
    /// down to what's no form of an adapter's, whose type says it, an
    /// array's iterator a variable holds or a type parameter's (#851).
    pub(in crate::lower) fn knows_its_back_of(&self, value: &Expr, ty: ty::Ty<'tcx>) -> bool {
        self.knows_its_back_beneath(value, ty, false)
    }

    /// `knows_its_back_of`'s, `wrapped` where a form holds `value`: a
    /// variable of an array's iterator's type holds a JS iterator only where
    /// a form holds it, `$closing(it)`; alone it may be the array itself.
    pub(in crate::lower) fn knows_its_back_beneath(&self, value: &Expr, ty: ty::Ty<'tcx>, wrapped: bool) -> bool {
        if crate::lower::iterators::knows_its_back_by_form(value) {
            return true;
        }
        let held = Self::held_by_holding(value).map(|held| (held, true));
        match held.or_else(|| crate::lower::iterators::back_source(value)) {
            Some((source, same)) => (self.back_source_ty(ty, same))
                .is_some_and(|source_ty| self.knows_its_back_beneath(source, source_ty, true)),
            None => {
                // Not std's compiled struct, which has a back only given it,
                // `$knowing`'s (#856).
                matches!(value.kind, js::ExprKind::Var(_))
                    && self.has_its_back_by_type(ty)
                    && (wrapped || !self.steps_an_array(ty))
                    && !self.compiled_struct(self.reveal(ty), value)
            }
        }
    }

    /// The type of what a form of type `ty` holds (`back_source`'s): its
    /// own, or its source's.
    pub(in crate::lower) fn back_source_ty(&self, ty: ty::Ty<'tcx>, same: bool) -> Option<ty::Ty<'tcx>> {
        let ty = self.reveal(ty.peel_refs());
        match same {
            true => Some(ty),
            false => crate::lower::recognition::back_stepped_source(self.tcx, ty)
                .or_else(|| crate::lower::recognition::enumerated_source(self.tcx, ty))
                .or_else(|| crate::lower::recognition::counted_source(self.tcx, ty).map(|(source, _)| source))
                .or_else(|| crate::lower::recognition::stepped_by_source(self.tcx, ty))
                .map(|source| self.reveal(source.peel_refs())),
        }
    }

    /// Whether a value of `ty` has a back by its type alone: an array's
    /// iterator, or one its type's structure says knows it.
    pub(in crate::lower) fn has_its_back_by_type(&self, ty: ty::Ty<'tcx>) -> bool {
        let outer = self.asking_made.replace(true);
        let knows = self.steps_an_array(ty) || self.knows_its_back(ty);
        self.asking_made.set(outer);
        knows
    }
}
