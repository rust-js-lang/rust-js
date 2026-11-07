//! A loop's walk (ADR 0576): what's known as each turn starts, the join of
//! what's known as it's entered and as each turn goes round again, until
//! that holds; and a `for`'s variable, in its range, `0..xs.len()`, or an
//! `enumerate()`'s index, below its slice's length (#363).

use super::known::{Below, Bounds, Known, Range, by_value, either};
use super::{Slice, Target, Walk};
use crate::lower::body_queries::{ForLoop, strip};
use crate::lower::fn_def;
use crate::lower::recognition::{is_iterator_enumerate, is_iterator_rev, is_iterator_step_by, is_slice_items};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::{ExprId, ExprKind, Pat, PatKind};

impl<'a, 'tcx> Walk<'a, 'tcx> {
    /// A loop of `body`, its scope the innermost: walked from what's known
    /// at its start each time round, the join of what's known as it's
    /// entered and as each turn goes round again, until that holds. Past
    /// it, what's known where it's left: its scope's `break`s, joined in
    /// where the scope ends. `each`, a `for`'s variable, is set as each
    /// turn starts, and its iterator, run out, leaves where a turn starts.
    pub(super) fn walk_loop(
        &mut self,
        body: ExprId,
        each: Option<(&Pat<'tcx>, Option<Range>)>,
        known: &mut Option<Known>,
    ) {
        let scope = self.targets.len().checked_sub(1);
        let entry = known.take();
        let mut head = entry.clone();
        for round in 0.. {
            let mut here = head.clone();
            if let Some((pat, value)) = &each {
                self.forget_bound(pat, &mut here);
                // Its item, or an `enumerate()`'s index, `(i, x)`'s `i` (#363).
                let index = match &pat.kind {
                    PatKind::Leaf { subpatterns } => (subpatterns.iter())
                        .find(|field| field.field.index() == 0)
                        .map(|field| &field.pattern),
                    _ => Some(*pat),
                };
                if let Some(PatKind::Binding {
                    var,
                    mode: BindingMode(ByRef::No, _),
                    subpattern: None,
                    ..
                }) = index.map(|index| &index.kind)
                    && let (Some(value), Some(k)) = (value, here.as_mut())
                {
                    k.vars.insert(*var, value.clone());
                }
            }
            self.expr(body, &mut here);
            let continued = scope.and_then(|at| self.targets[at].continues.take());
            let mut next = either(entry.clone(), either(here, continued));
            // Past a few rounds, what still changes is forgotten: what's
            // left holds. An integer still growing is taken first as at
            // most one below its type's most, where its `+ 1` can't wrap,
            // a counter's below a length; growing past that, anything
            // (ADR 0632). One found within what it was taken as, a
            // counter's below a string's length, keeps it: it holds
            // (ADR 0682).
            if round >= 3
                && let (Some(next), Some(head)) = (next.as_mut(), &head)
            {
                let most = &self.most;
                let length_most = &self.length_most;
                let vars = std::mem::take(&mut next.vars);
                next.vars = (vars.into_iter())
                    .filter_map(|(v, r)| {
                        let was = head.vars.get(&v)?;
                        if was.join(&r) == *was {
                            return Some((v, was.clone()));
                        }
                        // Still at most what it was by a length, a counter
                        // below `xs.len()`: at most that, where the length
                        // is its most (ADR 0701).
                        let below = (was.below)
                            .filter_map(|s, x| r.below.get(s).filter(|&y| x.join(y) == x).map(|_| (s.clone(), x)));
                        // One bound by a length is at most what that bound
                        // is, up to its type's most: the length holds it,
                        // not a `+ 1` that can't wrap (ADR 0709).
                        let type_most = *most.get(&v)?;
                        // A bound by another's value is at most what that
                        // one's type holds.
                        let base_most = |s: &Slice| match by_value(s) {
                            true => most.get(&s.1).copied(),
                            false => length_most.get(s).copied(),
                        };
                        let hi = (below.iter())
                            .map(|(s, x)| type_most.min(x.most(base_most(s).unwrap_or(u32::MAX as i128))))
                            .min()
                            .unwrap_or(type_most - 1);
                        (was.hi < hi && r.hi <= hi && r.lo >= was.lo).then(|| {
                            let widened = Range { lo: was.lo, hi, below };
                            (v, widened)
                        })
                    })
                    .collect();
                next.lengths.retain(|s, l| head.lengths.get(s) == Some(l));
            }
            if next == head {
                break;
            }
            // Each round past widening forgets something; one that never
            // settles is a bug, and nothing found is trusted.
            if round == 64 {
                self.lost = true;
                break;
            }
            head = next;
        }
        *known = match each {
            Some(_) => head,
            None => None,
        };
    }

    /// `for pat in head { body }`. Of a range, `0..xs.len()`, its variable is
    /// in it, and below the slice's length where the body never changes the
    /// slice: the range's end is its length as the loop began.
    pub(super) fn walk_for(&mut self, for_loop: ForLoop<'a, 'tcx>, known: &mut Option<Known>) {
        let thir = self.thir;
        let mut head = strip(thir, for_loop.head);
        // A range's `rev()`, its items the same, from its end (#356); and
        // its `step_by(n)`, some of them, each in it (rd260).
        while let ExprKind::Call { fun, ref args, .. } = thir[head].kind
            && let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty)
        {
            match args[..] {
                [range] if is_iterator_rev(self.tcx, def_id) => head = strip(thir, range),
                [range, n] if is_iterator_step_by(self.tcx, def_id) => {
                    self.expr(n, known);
                    head = strip(thir, range);
                }
                _ => break,
            }
        }
        // A slice's items' `enumerate()`, its index below the slice's
        // length, which the slice's `iter()` holds borrowed (#363).
        let enumerated = match thir[head].kind {
            ExprKind::Call { fun, ref args, .. }
                if let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty)
                    && is_iterator_enumerate(self.tcx, def_id)
                    && let [items] = args[..]
                    && let ExprKind::Call {
                        fun: items_fun,
                        args: ref of,
                        ..
                    } = thir[strip(thir, items)].kind
                    && let Some((items_def, _)) = fn_def(thir[strip(thir, items_fun)].ty)
                    && is_slice_items(self.tcx, items_def)
                    && let [slice] = of[..] =>
            {
                self.slice(slice).map(|slice| (slice, self.length_most_of(items_fun)))
            }
            _ => None,
        };
        // Its start and end, and how far below its end its last item is:
        // `a..b`'s 1, `a..=b`'s, `RangeInclusive::new(a, b)`, 0.
        let bounds = match thir[head].kind {
            ExprKind::Adt(ref range) if self.tcx.is_lang_item(range.adt_def.did(), LangItem::Range) => {
                let [start, end] = &range.fields[..] else {
                    unreachable!("a range's two fields")
                };
                Some((start.expr, end.expr, 1))
            }
            ExprKind::Call { fun, ref args, .. }
                if let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty)
                    && self.tcx.is_lang_item(def_id, LangItem::RangeInclusiveNew)
                    && let [start, end] = args[..] =>
            {
                Some((start, end, 0))
            }
            _ => None,
        };
        let body = thir[for_loop.body].span.source_callsite();
        let each = match bounds {
            Some((start_at, end_at, past)) => {
                let start = self.expr(start_at, known);
                let end = self.expr(end_at, known);
                let end = end.map(|r| self.bounded_by_itself(r, end_at));
                start.zip(end).map(|(start, end)| Range {
                    lo: start.lo,
                    hi: end.hi - past,
                    below: (end.below)
                        .filter_map(|slice, b| Some((self.unchanged_in(body, slice.clone())?, b.offset(-past)))),
                })
            }
            None => {
                self.expr(for_loop.head, known);
                enumerated.map(|(slice, most)| Range {
                    lo: 0,
                    hi: most - 1,
                    below: (self.unchanged_in(body, slice))
                        .map(|slice| Bounds::of(slice, Below::LENGTH.offset(-1)))
                        .unwrap_or_default(),
                })
            }
        };
        self.targets.push(Target {
            scope: for_loop.scope,
            breaks: None,
            continues: None,
        });
        self.walk_loop(for_loop.body, Some((for_loop.pat, each)), known);
        let target = self.targets.pop().expect("pushed");
        *known = either(known.take(), target.breaks);
    }

    /// `slice`, where nothing in `body`, a loop's, changes it: what a bound
    /// by its length holds of each time round.
    fn unchanged_in(&self, body: rustc_span::Span, slice: Slice) -> Option<Slice> {
        let changed = (self.changes.get(&slice.1)).is_some_and(|at| at.iter().any(|at| body.contains(*at)));
        (!changed).then_some(slice)
    }
}
