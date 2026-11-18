//! What a call tells the MIR intervals of what it gives (ADR 1137): std's
//! lengths and `is_empty()`, a range's `into_iter()` and `next()`, and
//! what else the walk knows a call's value to be, by recognition.

use super::*;
use crate::lower::recognition::{collects_into_vec, makes_a_split};
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
            .map(|of| state.root(of));
        // A `RefCell`'s `borrow()` and a `Ref`'s `Deref`, copies of the cell
        // (`cells`, #1741).
        self.borrowed_cell(func, of, local, after);
        // As recognition takes them (`slice_length`), never by name.
        if let Some((callee, callee_args)) = func.const_fn_def()
            && let Some(of) = of
        {
            match slice_length(self.tcx, callee, callee_args) {
                Some(SliceLength::Len) => {
                    after.lens.insert(local, of);
                    // At most what an allocation holds of its items.
                    let receiver = args.first().map(|arg| arg.node.ty(&*self.body, self.tcx));
                    if let Some(most) = receiver.and_then(|ty| self.length_of_collection(ty)) {
                        after.intervals[local] = Some(most);
                    }
                }
                Some(SliceLength::IsEmpty) => {
                    after.empties.insert(local, of);
                }
                // An index `v[i]` gave its item at: below the length, as it
                // panics otherwise, so a later `v[i]` or a drain to it is in
                // bounds (ADR 0881's, #1815).
                Some(SliceLength::Index) => {
                    if let Some(i) = (args.get(1))
                        .and_then(|arg| arg.node.place())
                        .and_then(|p| p.as_local())
                    {
                        after.related(i, Term::Len(of), true);
                    }
                }
                _ => {}
            }
        }
        // A `str::split` or `rsplit` made: until it's stepped, a `&mut` of
        // it lent, which forgets it, what it gives is never empty, a text
        // without its separator, `""` too, given whole. Its `collect()` into
        // a `Vec`, given it straight, holds one item at least (#1860). Not a
        // `split_terminator`'s, a `split_inclusive`'s or a `splitn`'s, which
        // may give none.
        if let Some((callee, _)) = func.const_fn_def()
            && makes_a_split(self.tcx, callee)
        {
            after.splits.insert(local);
        }
        if let Some((callee, _)) = func.const_fn_def()
            && collects_into_vec(self.tcx, callee, self.body.local_decls[local].ty)
            && let Some(Operand::Move(split)) = args.first().map(|arg| &arg.node)
            && let Some(split) = split.as_local()
            && state.splits.contains(&split)
        {
            after.least.insert(local, 1);
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
                        // Its length one more, which never wraps: what was
                        // at most it is below it, `v[n]` of `n = v.len()`
                        // before the push (ADR 1168, #1948).
                        let at_most: Vec<Local> = (state.below.iter())
                            .filter(|&&(_, y, _)| y == Term::Len(root))
                            .map(|&(x, _, _)| x)
                            .chain((state.lens.iter()).filter(|&(_, &of)| of == root).map(|(&n, _)| n))
                            .collect();
                        for x in at_most {
                            after.related(x, Term::Len(root), true);
                        }
                    }
                }
                Some(SliceLength::Len) if state.pushed.contains(&given) => {
                    let receiver = args.first().map(|arg| arg.node.ty(&*self.body, self.tcx));
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
        // `usize::from(apple)` of a `bool`: 0 or 1 (#1863).
        if let Some((callee, callee_args)) = func.const_fn_def()
            && integer_from_bool(self.tcx, callee, callee_args)
        {
            after.intervals[local] = Some(Interval { lo: 0, hi: 1 });
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
                let Some(x) = o.place().and_then(|p| p.as_local()) else {
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
            && is_rest_value(self.tcx, given.node.ty(&*self.body, self.tcx))
            && let Some(of) = given.node.place().and_then(|p| p.as_local())
        {
            after.copies.insert(local, state.root(of));
        }
        // A slice's iterator, whatever steps it: the slice it steps, which
        // a `position()` of it is an index of (#1945).
        if let Some((callee, callee_args)) = func.const_fn_def()
            && range_step(self.tcx, callee, callee_args) == Some(RangeStep::SliceIter)
            && let Some(of) = (args.first())
                .and_then(|arg| arg.node.place())
                .and_then(|p| p.as_local())
        {
            after.iterates.insert(local, state.root(of));
        }
        // A slice's iterator, `s.iter()` or `s.iter_mut()`: of the
        // slice it's of, by what it's given, a reborrow of it.
        if let Some((callee, callee_args)) = func.const_fn_def()
            && range_step(self.tcx, callee, callee_args) == Some(RangeStep::SliceIter)
            && self.stepped.contains(&local)
            && let Some(of) = (args.first())
                .and_then(|arg| arg.node.place())
                .and_then(|p| p.as_local())
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
            && let Some(index) = index.node.place().and_then(|p| p.as_local())
            && let Some(most) = self.length_of_collection(slice.node.ty(&*self.body, self.tcx))
        {
            after.gets.insert(local, (state.root(index), most.hi - 1));
        }
        // A thread-local's `with`, `with_borrow` or `with_borrow_mut`
        // gives what its closure gives: of one whose `Some` knows a capture
        // by shared reference below a length, `targets.get(i).cloned()`,
        // what it refers to here, `i` (#1701 C).
        if let Some((callee, _)) = func.const_fn_def()
            && local_key_with(self.tcx, callee)
            && let Some(closure) = args.get(1).and_then(|arg| arg.node.place()).and_then(|p| p.as_local())
            && let Some((made, referred)) = self.made_closure(closure)
            && let Some((field, most)) = closure_result(self.tcx, made)
            && let Some(&Some(index)) = referred.get(field)
        {
            after.gets.insert(local, (state.root(index), most));
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
        // A slice iterator's `position()` or `rposition()`, of what it's
        // lent: its `Some`'s index below the slice's length (#1945).
        if let Some((callee, _)) = func.const_fn_def()
            && slice_position(self.tcx, callee)
            && let Some(&slice) = (args.first())
                .and_then(|arg| arg.node.place())
                .and_then(|p| p.as_local())
                .and_then(|it| state.iterates.get(&it))
        {
            after.positions.insert(local, (slice, 1));
        }
        // A `?` of one, `Try::branch(o)`: the index its `Continue`'s.
        if let Some((callee, _)) = func.const_fn_def()
            && crate::lower::mir::result_use::is_try_branch(self.tcx, callee)
            && let Some(&(slice, 1)) = (args.first())
                .and_then(|arg| arg.node.place())
                .and_then(|p| p.as_local())
                .and_then(|o| state.positions.get(&o))
        {
            after.positions.insert(local, (slice, 0));
        }
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
                (Some(RangeStep::IntoIter | RangeStep::Rev | RangeStep::StepBy | RangeStep::Skip), Held::Range)
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

/// Whether `callee` is `Iterator::position` or `Iterator::rposition`,
/// whose `Some` is an index of an item the iterator gave (#1945).
fn slice_position(tcx: rustc_middle::ty::TyCtxt<'_>, callee: rustc_span::def_id::DefId) -> bool {
    use crate::lower::recognition::{StdItem, std_item, trait_method};
    let iterator = std_item(tcx, StdItem::Iterator);
    callee == trait_method(tcx, iterator, "position") || callee == trait_method(tcx, iterator, "rposition")
}
