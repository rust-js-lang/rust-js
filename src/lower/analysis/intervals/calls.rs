//! What a call tells the MIR intervals of what it gives (ADR 1137): std's
//! lengths and `is_empty()`, a range's `into_iter()` and `next()`, and
//! what else the walk knows a call's value to be, by recognition.

use super::*;
use rustc_span::Spanned;

impl<'tcx> Walk<'_, 'tcx> {
    /// What `func`'s call of `args` tells of what it gives, `local`, into
    /// `after`, what's known past it; `state` what's known before.
    pub(super) fn called(
        &self,
        func: &Operand<'tcx>,
        args: &[Spanned<Operand<'tcx>>],
        local: Local,
        state: &State,
        after: &mut State,
    ) {
        // std's `len()` and `is_empty()` of a slice, a `Vec` or
        // text, by what it's given, `&v` the `v` it lends.
        let of = (args.first())
            .and_then(|arg| arg.node.place())
            .and_then(|p| p.as_local())
            .filter(|of| !self.lent.contains(of))
            .map(|of| state.root(of));
        // As recognition takes them (`slice_length`), never by name.
        if let Some((callee, callee_args)) = func.const_fn_def()
            && let Some(of) = of
        {
            match slice_length(self.tcx, callee, callee_args) {
                Some(SliceLength::Len) => {
                    after.lens.insert(local, of);
                    // At most what an allocation holds of its items.
                    let receiver = args.first().map(|arg| arg.node.ty(self.body, self.tcx));
                    if let Some(most) = receiver.and_then(|ty| self.length_of_collection(ty)) {
                        after.intervals[local] = Some(most);
                    }
                }
                Some(SliceLength::IsEmpty) => {
                    after.empties.insert(local, of);
                }
                _ => {}
            }
        }
        // A `Vec`'s `push`: what it's given a `&mut` of holds an item at
        // least (`pushes`); and its `len()`, of one that does, is at least
        // one, `prev_length - 1` of `prev_length = properties.len()` past a
        // push (#1700).
        let given = args.first().and_then(|arg| arg.node.place()).and_then(|p| p.as_local());
        if let Some((callee, callee_args)) = func.const_fn_def()
            && let Some(given) = given
        {
            match slice_length(self.tcx, callee, callee_args) {
                Some(SliceLength::Push) => {
                    if let Some(&root) = state.lending.get(&given) {
                        after.pushed.insert(root);
                    }
                }
                Some(SliceLength::Len) if state.pushed.contains(&given) => {
                    let receiver = args.first().map(|arg| arg.node.ty(self.body, self.tcx));
                    if let Some(most) = receiver.and_then(|ty| self.length_of_collection(ty)) {
                        after.intervals[local] = Some(Interval {
                            lo: most.lo.max(1),
                            ..most
                        });
                    }
                }
                _ => {}
            }
        }
        // An integer's `min` or `max`: between its operands, and
        // below what the one, or both, are below, a length held
        // among them; `xs[i.min(xs.len() - 1)]`.
        if let Some((callee, callee_args)) = func.const_fn_def()
            && let Some(max) = integer_min_max(self.tcx, callee, callee_args)
            && let [a, b] = args
            && let (Some(ia), Some(ib)) = (self.operand(&a.node, state), self.operand(&b.node, state))
        {
            after.intervals[local] = Some(match max {
                false => Interval {
                    lo: ia.lo.min(ib.lo),
                    hi: ia.hi.min(ib.hi),
                },
                true => Interval {
                    lo: ia.lo.max(ib.lo),
                    hi: ia.hi.max(ib.hi),
                },
            });
            let relations = |o: &Operand<'tcx>| -> IndexSet<(Term, bool)> {
                let Some(x) = o.place().and_then(|p| p.as_local()).filter(|x| !self.lent.contains(x)) else {
                    return IndexSet::default();
                };
                let x = state.root(x);
                let mut out: IndexSet<(Term, bool)> = (state.below.iter())
                    .filter(|&&(of, _, _)| of == x)
                    .map(|&(_, y, strict)| (y, strict))
                    .collect();
                out.insert((state.length_of(x).map_or(Term::Int(x), Term::Len), false));
                out
            };
            let (ra, rb) = (relations(&a.node), relations(&b.node));
            let below: Vec<(Term, bool)> = match max {
                false => ra.union(&rb).copied().collect(),
                true => (ra.iter())
                    .filter_map(|&(y, s)| {
                        let there = match (rb.contains(&(y, true)), rb.contains(&(y, false))) {
                            (true, _) => Some(true),
                            (false, true) => Some(false),
                            _ => None,
                        }?;
                        Some((y, s && there))
                    })
                    .collect(),
            };
            for (y, strict) in below {
                if y != Term::Int(local) && y != Term::Len(local) {
                    after.below.insert((local, y, strict));
                }
            }
        }
        // Rest arguments' `Deref`, `&dyn js::Rest<T>`'s: the array
        // they are, the slice it gives of the same length.
        if let Some((callee, _)) = func.const_fn_def()
            && (self.tcx.trait_of_assoc(callee)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Deref))
            && let [given] = args
            && is_rest_value(self.tcx, given.node.ty(self.body, self.tcx))
            && let Some(of) = given
                .node
                .place()
                .and_then(|p| p.as_local())
                .filter(|of| !self.lent.contains(of))
        {
            after.copies.insert(local, state.root(of));
        }
        // A slice's iterator, `s.iter()` or `s.iter_mut()`: of the
        // slice it's of, by what it's given, a reborrow of it.
        if let Some((callee, callee_args)) = func.const_fn_def()
            && range_step(self.tcx, callee, callee_args) == Some(RangeStep::SliceIter)
            && self.stepped.contains(&local)
            && let Some(of) = (args.first())
                .and_then(|arg| arg.node.place())
                .and_then(|p| p.as_local())
                .filter(|of| !self.lent.contains(of))
        {
            after.ranges.insert(
                local,
                Ranged {
                    held: Held::Range,
                    least: 0,
                    end: End::Items(state.root(of)),
                },
            );
        }
        // An `Option`'s `cloned()` or `copied()` of a slice's `get(index)`:
        // `Some` where that was.
        if let Some((callee, _)) = func.const_fn_def()
            && option_copied(self.tcx, callee)
            && let Some(&got) = (args.first())
                .and_then(|arg| arg.node.place())
                .and_then(|p| p.as_local())
                .and_then(|of| state.gets.get(&of))
        {
            after.gets.insert(local, got);
        }
        // A slice's `get(index)`: its `Some` of an index below its length.
        if let Some((callee, callee_args)) = func.const_fn_def()
            && slice_get(self.tcx, callee, callee_args)
            && let [slice, index] = args
            && let Some(index) = index
                .node
                .place()
                .and_then(|p| p.as_local())
                .filter(|i| !self.lent.contains(i))
            && let Some(most) = self.length_of_collection(slice.node.ty(self.body, self.tcx))
        {
            after.gets.insert(local, (state.root(index), most.hi - 1));
        }
        // `a..=b`, `RangeInclusive::new(a, b)`'s, of integers.
        if let Some((callee, callee_args)) = func.const_fn_def()
            && self.tcx.is_lang_item(callee, LangItem::RangeInclusiveNew)
            && callee_args.types().next().is_some_and(|item| item.is_integral())
            && self.stepped.contains(&local)
            && let [start, end] = args
            && let (Some(start), Some(end)) = (self.operand(&start.node, state), self.value(&end.node, state))
        {
            after.ranges.insert(
                local,
                Ranged {
                    held: Held::Range,
                    least: start.lo,
                    end: End::AtMost(end),
                },
            );
        }
        // A JS string's `length`, a binding's, or its count of UTF-16
        // units, `s.encode_utf16().count()`: at most `2^31 - 1` (ADR
        // 0632); text's `len()`, its bytes, at most what an
        // allocation holds.
        if let Some((callee, callee_args)) = func.const_fn_def() {
            let js_string = reads_string_length(self.tcx, callee)
                || (callee_args.types().next()).is_some_and(|of| counts_utf16_units(self.tcx, callee, of));
            if js_string {
                after.intervals[local] = Some(Interval {
                    lo: 0,
                    hi: STRING_LENGTH_MOST,
                });
                // Of text, a length of it: its UTF-16 units, at most its
                // bytes, so what's below it is below them, `i` of `while
                // i + 1 < js::string::length(format)` round the loop.
                if reads_string_length(self.tcx, callee)
                    && let Some(of) = of
                {
                    after.lens.insert(local, of);
                }
            } else if text_length(self.tcx, callee) {
                after.intervals[local] = self.length(self.tcx.types.str_);
            }
        }
        // A range's `into_iter()`, itself, and its `next()`, what
        // it gives.
        let given = (args.first())
            .and_then(|arg| arg.node.place())
            .and_then(|p| p.as_local())
            .and_then(|of| state.ranges.get(&of).copied());
        if let Some((callee, callee_args)) = func.const_fn_def()
            && let Some(given) = given
        {
            match (range_step(self.tcx, callee, callee_args), given.held) {
                (Some(RangeStep::Enumerate), Held::Range)
                    if self.stepped.contains(&local) && matches!(given.end, End::Items(_)) =>
                {
                    if let End::Items(slice) = given.end {
                        after.ranges.insert(
                            local,
                            Ranged {
                                end: End::Indexes(slice),
                                ..given
                            },
                        );
                    }
                }
                (Some(RangeStep::IntoIter | RangeStep::Rev | RangeStep::StepBy), Held::Range)
                    if self.stepped.contains(&local) =>
                {
                    after.ranges.insert(local, given);
                }
                (Some(RangeStep::Next), Held::Lent) if !matches!(given.end, End::Items(_)) => {
                    after.ranges.insert(
                        local,
                        Ranged {
                            held: Held::Next,
                            ..given
                        },
                    );
                }
                _ => {}
            }
        }
    }
}
