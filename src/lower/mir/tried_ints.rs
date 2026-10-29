//! An integer's `try_from` whose `Result` is only tested and its `Ok`
//! taken, `let Ok(n) = usize::try_from(i) else { .. }`: which ones are
//! (`mir_tried_ints`, a fact `LocalFacts::tried_int` keeps), and the cut
//! each is, its range test and the number, `if (n < 0)`, as a person
//! writes it (ADR 0642, amended, #366).

use super::*;
use crate::lower::Num;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The blocks `LocalFacts::tried_int` says: each that ends in a call of
    /// an integer's `try_from` whose `Result`, written there alone, is read
    /// only by its discriminant and its `Ok`'s number.
    pub(super) fn mir_tried_ints(&self, body: &mir::Body<'tcx>) -> HashSet<BasicBlock> {
        struct Reads(Local, bool);
        impl<'tcx> Visitor<'tcx> for Reads {
            fn visit_place(&mut self, place: &Place<'tcx>, context: PlaceContext, _: mir::Location) {
                if place.local != self.0 {
                    return;
                }
                self.1 &= match (context, place.projection.as_slice()) {
                    (PlaceContext::NonUse(_) | PlaceContext::MutatingUse(MutatingUseContext::Call), _) => true,
                    // Its discriminant, or a `let`'s mention of it.
                    (
                        PlaceContext::NonMutatingUse(
                            NonMutatingUseContext::Inspect | NonMutatingUseContext::PlaceMention,
                        ),
                        [],
                    ) => true,
                    (
                        PlaceContext::NonMutatingUse(NonMutatingUseContext::Copy | NonMutatingUseContext::Move),
                        [PlaceElem::Downcast(_, variant), PlaceElem::Field(..)],
                    ) => variant.as_u32() == 0,
                    _ => false,
                };
            }
        }
        let calls = |local: Local| {
            (body.basic_blocks.iter())
                .filter(|data| matches!(&data.terminator().kind, TerminatorKind::Call { destination, .. } if destination.local == local))
                .count()
        };
        (body.basic_blocks.iter_enumerated())
            .filter(|(_, data)| {
                let TerminatorKind::Call { func, destination, .. } = &data.terminator().kind else {
                    return false;
                };
                let tried = fn_def(func.ty(&body.local_decls, self.tcx))
                    .and_then(|(def_id, args)| self.recognition().classify(def_id, args))
                    .is_some_and(|known| matches!(known, Std::TryFromInt { .. }));
                // Not the return place, which `return` reads whole.
                let local = destination.local;
                tried && destination.projection.is_empty() && local != mir::RETURN_PLACE && calls(local) == 1 && {
                    let mut reads = Reads(local, true);
                    reads.visit_body(body);
                    reads.1
                }
            })
            .map(|(block, _)| block)
            .collect()
    }

    /// The types a `try_from` of `from` into `target` is a cut of: of
    /// integers but a `NonZero`, whose `0` is its error too (ADR 0177).
    pub(super) fn tried_int_types(&self, target: Ty<'tcx>, from: Ty<'tcx>) -> Option<(Num, Num)> {
        let (from, to) = (Num::of(from.peel_refs())?, Num::of(target)?);
        let integers = !from.float() && !to.float();
        (integers && !crate::lower::recognition::is_non_zero_ty(target)).then_some((from, to))
    }

    /// The cut of a `try_from` `tried_int_types` takes, given the number,
    /// a `from`, into a `to`: its test, that it's out of range, and the
    /// number, held first where what it's of may change between.
    pub(super) fn tried_int(
        &mut self,
        state: &mut State<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        (from, to): (Num, Num),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let n = self.value_expr(values.next().expect("the number"), span)?;
        // Read at the test and where its `Ok` is: what reads the same
        // there, or held first.
        let mut plain = n.reads_same();
        n.visit_vars(&mut |name| plain &= self.plain_local(name));
        let n = match plain {
            true => n,
            false => {
                self.flush(state, out)?;
                self.spill("value", n, out)
            }
        };
        let out_of = crate::lower::std_types::number::int_out_of(&n, from, to);
        let n = crate::lower::std_types::number::int_as(n, from, to);
        Ok(Value::Cut(out_of, n, VariantIdx::from_u32(0)))
    }
}
