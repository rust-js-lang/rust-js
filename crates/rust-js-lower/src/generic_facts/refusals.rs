//! What generic code is refused of a call's values where it can't hold
//! them as its caller does (`generic_facts.rs`' crossing, ADR 1115): a
//! `RefCell` guard's `&mut` it may write whole (#2475), and an iterator JS
//! holds as no object, which it holds in a place of its own, inside what
//! it takes or gives or among another iterator's items (#2255, #2534).
//! Each route of a call asks `refuse_placed_within`; `crossing_into` asks
//! `refuse_guard_written_whole`. Each is a stopgap, its cell a
//! `refusal::Crossing` the issue it names lifts (ADR 1201).

use super::writes_through_its_box;
use crate::recognition::StdItem;
use crate::{FnCx, R};
use rustc_middle::ty::{self, Mutability, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `RefCell` guard's `&mut`, the caller's `instantiated`, refused where
    /// generic code's `declared` `&mut T` may write it whole, `*slot = v`
    /// (`writes_through_its_box`), given another guard of its type to write,
    /// one of the call's `others` holding one, `put(&mut g, h)`'s `h`: Rust's
    /// write drops the guard and makes the variable another's, where the JS
    /// of a guard, the cell's own object, is written in place, its cell given
    /// another's contents and the other's borrow never given back (#2475),
    /// until a form that rebinds the caller's variable exists. Generic code
    /// given no other, `fn bump<P: DerefMut>(p: &mut P)`, writes only what
    /// the guard guards.
    pub(super) fn refuse_guard_written_whole(
        &self,
        (declared, instantiated): (Ty<'tcx>, Ty<'tcx>),
        mut others: impl Iterator<Item = Ty<'tcx>>,
        span: Span,
    ) -> R<()> {
        if let ty::Ref(_, _, Mutability::Mut) = *declared.kind()
            && writes_through_its_box(declared)
            && let ty::Ref(_, guard, Mutability::Mut) = *instantiated.kind()
            && crate::recognition::is_cell_guard(self.tcx, guard)
            && let guard = self.tcx.erase_and_anonymize_regions(guard)
            && others.any(|other| {
                (self.tcx.erase_and_anonymize_regions(other).walk()).any(|part| part.as_type() == Some(guard))
            })
        {
            return Err(self.unconverted(span, crate::refusal::Crossing::GuardArgument, &format!("a `RefCell` guard's `&mut`, `&mut {guard}`, given to generic code's `{declared}`, which may write the guard whole, giving its variable another cell's (#2475)"),
            ));
        }
        Ok(())
    }

    /// A call of generic code `id`, its instance of `args`, refused where
    /// an iterator generic code holds in a place of its own is inside what
    /// it takes or gives, a `Vec<I>`'s, which its caller holds as it is
    /// (`placed_within`, #2255): each route of a call asks it.
    pub fn refuse_placed_within(&self, (id, args): (DefId, ty::GenericArgsRef<'tcx>), span: Span) -> R<()> {
        let sig = (self.tcx)
            .instantiate_bound_regions_with_erased(self.tcx.fn_sig(id).instantiate_identity().skip_normalization());
        for declared in sig.inputs_and_output.iter() {
            if let Some(placed) = self.placed_within(id, declared, self.instantiated(declared, args)) {
                return Err(self.unconverted(span, crate::refusal::Crossing::PlacedArgument, &format!(
                        "a `{placed}` inside a `{declared}` of generic code's, which holds it in a place of its own, `{{ value }}`, where its caller holds it as it is (#2255)"
                    ),
                ));
            }
        }
        self.refuse_placed_item((id, args), span)
    }

    /// A call of generic code `id`, its instance of `args`, refused where a
    /// type parameter it holds in a place of its own (`holds_in_place`) is
    /// also another's items, `S: Iterator<Item = I>`, `Self: Iterator<Item
    /// = J>`, an `IntoIterator`'s, or a part of them, `Item = (I, u8)`:
    /// those come as their caller holds them, never in a place, where its
    /// dictionaries read the place, `.value` of a bare string (#2534), until
    /// each item is made a place where it crosses (#2535).
    fn refuse_placed_item(&self, (id, args): (DefId, ty::GenericArgsRef<'tcx>), span: Span) -> R<()> {
        let tcx = self.tcx;
        let items = [StdItem::Iterator, StdItem::IntoIterator]
            .map(|item| crate::recognition::trait_method(tcx, crate::recognition::std_item(tcx, item), "Item"));
        for (clause, _) in tcx.clauses_of(id).instantiate_identity(tcx) {
            let Some(p) = clause.skip_normalization().as_projection_clause() else {
                continue;
            };
            let p = p.skip_binder();
            let Some(term) = p.term.as_type() else { continue };
            if !items.contains(&p.def_id()) {
                continue;
            }
            // Its type parameters, not one a projection names, `I::Item`,
            // nor the one whose items they are.
            let mut walk = term.walk();
            while let Some(part) = walk.next() {
                let Some(param) = part.as_type() else { continue };
                if matches!(param.kind(), ty::Alias(..)) {
                    walk.skip_current_subtree();
                    continue;
                }
                if !matches!(param.kind(), ty::Param(_)) || param == p.self_ty() {
                    continue;
                }
                let instantiated = self.instantiated(param, args);
                if self.holds_in_place((id, param, instantiated)) {
                    return Err(self.unconverted(span, crate::refusal::Crossing::PlacedItem, &format!(
                            "a `{instantiated}` generic code holds in a place of its own, `{{ value }}`, as its `{param}`, which it's also given as `{}`'s items, as their caller holds them (#2534, #2535)",
                            p.self_ty()
                        ),
                    ));
                }
            }
        }
        // And items no type parameter names, stepped through their own
        // bound, `S::Item: Iterator`: they come as their caller holds them,
        // a fieldless enum's string, where the bound's `next`, of a `&mut`
        // that's a cell, writes the place's `value` (#2541, #2535).
        for (clause, _) in tcx.clauses_of(id).instantiate_identity(tcx) {
            let Some(bound) = clause.skip_normalization().as_trait_clause() else {
                continue;
            };
            let bound = bound.skip_binder();
            let item = bound.self_ty();
            if !crate::recognition::is_iterator_trait(tcx, bound.def_id())
                || !matches!(*item.kind(), ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if items.contains(&def_id))
            {
                continue;
            }
            let instantiated = self.instantiated(item, args);
            if self.placed_iterator(instantiated) {
                return Err(self.unconverted(
                    span,
                    crate::refusal::Crossing::PlacedItem,
                    &format!(
                        "a `{instantiated}`, which JS holds as no object, given as `{item}`, stepped through its `{}` as generic code holds it, in a place of its own, `{{ value }}`, where its items come as their caller holds them (#2541, #2535)",
                        tcx.def_path_str(bound.def_id())
                    ),
                ));
            }
        }
        Ok(())
    }
}
