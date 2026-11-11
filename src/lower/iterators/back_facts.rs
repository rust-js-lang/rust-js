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
    /// Whether `ty` is the type of what the call being lowered makes, into a
    /// local back_stepped's `stepped_locals` says the crate steps from its
    /// back, lent or given (ADR 0545, #851): by the value, its maker's MIR
    /// identity, not its type, which another value of it need not share. Of
    /// what's asked of a value made before, never: its mark says, carried by
    /// its local, `has_a_back`'s.
    pub(in crate::lower) fn stepped_from_its_back(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = self.tcx.erase_and_anonymize_regions(ty.peel_refs());
        self.krate.back_stepped.typed.contains(&ty)
            // What this generic function makes of its own parameters, given
            // where a hint is asked (`hint_asked.rs`, #1240).
            || (self.krate.hinted.knowing_in).contains(&(self.tcx.typeck_root_def_id(self.frame.owner), ty))
            || !self.asking_made.get()
                && (self.made_into.iter().chain(&self.made_from)).any(|&(local, made)| {
                    let local = (self.krate.back_stepped.borrows)
                        .get(&(self.frame.owner, local))
                        .copied()
                        .unwrap_or(local);
                    made == ty && self.local_stepped_from_its_back(local).is_some()
                })
    }

    /// Whether `ty` is an adapter whose maker makes it knowing its back
    /// where it's stepped so (#851): a map's, a filter's, a `filter_map`'s,
    /// an `inspect`'s, a chain's, a copy's, an enumeration's, a flatten's or a
    /// `rev()`'s, so a variable or a field that holds one is marked by it
    /// (`structure.rs`, `places.rs`). Not a zip's, a `skip(n)`'s, a
    /// `take(n)`'s nor a `step_by(n)`'s, whose maker makes none so: what
    /// steps it from its back makes it so where it's made, `$zipEnds`,
    /// `$taking`, `$skipping`, `$stepByEnds` (ADR 0542), or it's std's struct
    /// where MIR holds it (#718); nor the crate's own iterator.
    pub(in crate::lower) fn made_knowing_its_back_by_its_maker(&self, ty: ty::Ty<'tcx>) -> bool {
        use crate::lower::recognition as r;
        r::mapped_source(self.tcx, ty).is_some()
            || r::filtered_source(self.tcx, ty).is_some()
            || r::filter_mapped_source(self.tcx, ty).is_some()
            || r::inspected_source(self.tcx, ty).is_some()
            || r::chained_sources(self.tcx, ty).is_some()
            || r::copied_source(self.tcx, ty).is_some()
            || r::cloned_source(self.tcx, ty).is_some()
            || r::enumerated_source(self.tcx, ty).is_some()
            || r::flattened_source(self.tcx, ty).is_some()
            || r::reversed_source(self.tcx, ty).is_some()
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

    /// Whether a string's `Chars` of type `ty` is asked more than steps
    /// from its front (#1186): cloned, its type one the crate clones; or,
    /// of any `Chars` the crate has, stepped from its back, asked its rest,
    /// its hint or its `Debug`, made a `dyn`, or given to code that isn't
    /// std's (`chars_whole`). Such a one is the runtime's `$chars(s)`, which
    /// knows its back, its rest and its hint; another JS's own string
    /// iterator, `s[Symbol.iterator]()`.
    pub(in crate::lower) fn chars_asked_whole(&self, ty: ty::Ty<'tcx>) -> bool {
        self.krate.chars_whole
            || self
                .krate
                .cloned
                .contains(&self.tcx.erase_and_anonymize_regions(ty.peel_refs()))
    }

    /// Whether a string's `chars()` or `char_indices()` of type `ty` knows
    /// its back, `None` of another type (#1186): where it's asked more than
    /// its front's steps (`chars_asked_whole`), so that what's made of it
    /// is made knowing its back, `$mapEnds` and the like, and otherwise
    /// JS's own adapters.
    pub(in crate::lower) fn text_knows_its_back(&self, ty: ty::Ty<'tcx>) -> Option<bool> {
        let text = self.recognition();
        (text.is_str_chars(ty) || text.is_char_indices(ty)).then(|| self.chars_asked_whole(ty))
    }

    /// Whether a `take(n)`, a `skip(n)` or a `step_by(n)` of type `ty` is
    /// one generic code asks the hint of, made `$taking`, `$skipping` or
    /// `$stepBy` where it's made, as `hint_asked.rs` says (#1236).
    pub(in crate::lower) fn made_knowing_its_hint(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = self.tcx.erase_and_anonymize_regions(ty.peel_refs());
        let owner = self.tcx.typeck_root_def_id(self.frame.owner);
        self.krate.hinted.types.contains(&ty) || self.krate.hinted.counted_in.contains(&(owner, ty))
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
                    && !self.compiled_struct(ty, value)
            }
        }
    }

    /// The type of what a form of type `ty` holds (`back_source`'s): its
    /// own, or its source's.
    pub(in crate::lower) fn back_source_ty(&self, ty: ty::Ty<'tcx>, same: bool) -> Option<ty::Ty<'tcx>> {
        let ty = ty.peel_refs();
        match same {
            true => Some(ty),
            false => crate::lower::recognition::back_stepped_source(self.tcx, ty)
                .or_else(|| crate::lower::recognition::enumerated_source(self.tcx, ty))
                .or_else(|| crate::lower::recognition::counted_source(self.tcx, ty).map(|(source, _)| source))
                .or_else(|| crate::lower::recognition::stepped_by_source(self.tcx, ty))
                .map(|source| source.peel_refs()),
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

    /// Why what steps a `ty` from its back is refused, said after it: of a
    /// map whose JS is its source's (`as_is_map_because`); of what holds
    /// std's struct, an adapter a function gives, which its value is
    /// (#1226, the user's decision (A)), its follow-up; else nothing.
    pub(in crate::lower) fn back_refused_because(&self, ty: ty::Ty<'tcx>) -> &'static str {
        let as_is = self.as_is_map_because(ty);
        if !as_is.is_empty() {
            return as_is;
        }
        let compiled = (ty.walk().filter_map(|part| part.as_type()))
            .any(|part| part != ty && crate::lower::recognition::is_held_compiled(self.tcx, part));
        match compiled {
            true => ", which holds std's struct of an adapter a function gives (#1235)",
            false => "",
        }
    }

    /// Why a map refused as a double-ended or exact-sized iterator is,
    /// said after it: one whose function gives each item as it is,
    /// `map(|x| Some(x))`, is its source's JS (`mir/iter.rs`), so what
    /// steps it from its back or counts it finds no map to make knowing
    /// that (#1342); else nothing.
    pub(in crate::lower) fn as_is_map_because(&self, ty: ty::Ty<'tcx>) -> &'static str {
        match crate::lower::recognition::mapped_source(self.tcx, ty) {
            Some(_) => ", a map whose JS is its source's, its function giving each item as it is (#1342)",
            None => "",
        }
    }
}
