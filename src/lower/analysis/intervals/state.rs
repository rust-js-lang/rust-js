//! What the MIR intervals know at a point, and how two points' knowledge
//! joins (ADR 1137): each local's interval, the comparisons `bool`s hold,
//! copies, lengths, the relations `x < y`, each slice's least.

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Interval {
    pub(super) lo: i128,
    pub(super) hi: i128,
}

impl Interval {
    pub(super) fn join(self, other: Interval) -> Interval {
        Interval {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }
}

/// A comparison a `bool` local holds: `a op b`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Comparison {
    pub(super) op: BinOp,
    pub(super) a: Value,
    pub(super) b: Value,
}

/// What a comparison compares: a local or a constant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Value {
    Local(Local),
    Constant(i128),
}

/// What a relation's greater side is: a slice's length, by the local the
/// slice is (`Len`), or an integer local.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Term {
    Len(Local),
    Int(Local),
}

/// What's known at a point: each integer local's interval, none where it
/// isn't; each `bool` local's comparison; and each local that's a copy of
/// another neither has been written since, `_7 = copy _3`, which MIR
/// compares in its place, so a comparison is of what it copies, or a
/// reborrow of it, `_9 = &_1`, whose length is the same. And the small
/// relational part (ADR 1137, M2b): each local that holds a slice's
/// length, by the slice; each `x < y` and `x <= y` known, strict or not;
/// the least each slice's length is; and each `bool` that's a slice's
/// `is_empty()`.
#[derive(Clone, PartialEq)]
pub(super) struct State {
    pub(super) intervals: IndexVec<Local, Option<Interval>>,
    pub(super) comparisons: IndexMap<Local, Comparison>,
    pub(super) copies: IndexMap<Local, Local>,
    pub(super) lens: IndexMap<Local, Local>,
    pub(super) below: IndexSet<(Local, Term, bool)>,
    pub(super) least: IndexMap<Local, i128>,
    pub(super) empties: IndexMap<Local, Local>,
    /// A `for` loop's range of integers, by the local holding it, a `&mut`
    /// of it, or the `Option` its `next()` gave.
    pub(super) ranges: IndexMap<Local, Ranged>,
}

/// A range of integers `next()` alone steps (`stepped_ranges`): what its
/// start is at least, and its end, which never changes while it's known.
/// Each value its `next()` gives is at least the one and below the other.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Ranged {
    pub(super) held: Held,
    pub(super) least: i128,
    pub(super) end: Value,
}

/// What holds a range: a local, the range; a `&mut` of it, lent to its
/// `next()`; or what `next()` gave.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Held {
    Range,
    Lent,
    Next,
}

impl State {
    pub(super) fn join(&self, other: &State) -> State {
        let intervals = (self.intervals.iter().zip(other.intervals.iter()))
            .map(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => Some(a.join(*b)),
                _ => None,
            })
            .collect();
        let comparisons = (self.comparisons.iter())
            .filter(|&(local, comparison)| other.comparisons.get(local) == Some(comparison))
            .map(|(&local, &comparison)| (local, comparison))
            .collect();
        let both = |a: &IndexMap<Local, Local>, b: &IndexMap<Local, Local>| -> IndexMap<Local, Local> {
            (a.iter())
                .filter(|&(k, v)| b.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect()
        };
        let least = (self.least.iter())
            .filter_map(|(&k, &n)| other.least.get(&k).map(|&m| (k, n.min(m))))
            .collect();
        let ranges = (self.ranges.iter())
            .filter_map(|(&k, &a)| {
                let b = other.ranges.get(&k)?;
                (a.held == b.held && a.end == b.end).then_some((
                    k,
                    Ranged {
                        least: a.least.min(b.least),
                        ..a
                    },
                ))
            })
            .collect();
        State {
            intervals,
            comparisons,
            copies: both(&self.copies, &other.copies),
            lens: both(&self.lens, &other.lens),
            below: self.below.intersection(&other.below).copied().collect(),
            least,
            empties: both(&self.empties, &other.empties),
            ranges,
        }
    }

    /// `local` written: what it held, and each comparison of it, forgotten.
    pub(super) fn forget(&mut self, local: Local) {
        self.intervals[local] = None;
        self.comparisons.shift_remove(&local);
        self.comparisons
            .retain(|_, c| c.a != Value::Local(local) && c.b != Value::Local(local));
        self.copies.retain(|&copy, &mut of| copy != local && of != local);
        self.lens.retain(|&holder, &mut of| holder != local && of != local);
        self.below
            .retain(|&(x, y, _)| x != local && y != Term::Len(local) && y != Term::Int(local));
        self.least.shift_remove(&local);
        self.empties.retain(|&holder, &mut of| holder != local && of != local);
        self.ranges.shift_remove(&local);
        self.ranges.retain(|_, ranged| ranged.end != Value::Local(local));
    }

    /// What `local` is a copy or a reborrow of, else itself.
    pub(super) fn root(&self, local: Local) -> Local {
        self.copies.get(&local).copied().unwrap_or(local)
    }

    /// What's known of `local`, known too of what it copies and of each
    /// other copy of that: `i > 100` of `let mut i = start` tells `i`,
    /// though the comparison is of `start`, what `i` copies.
    pub(super) fn narrowed_to(&mut self, local: Local, interval: Interval) {
        let root = self.root(local);
        let copies: Vec<Local> = (self.copies.iter())
            .filter(|&(_, &of)| of == root)
            .map(|(&copy, _)| copy)
            .collect();
        for l in [local, root].into_iter().chain(copies) {
            self.intervals[l] = Some(interval);
        }
    }

    /// The slice whose length `local` holds, itself or what it copies.
    pub(super) fn length_of(&self, local: Local) -> Option<Local> {
        self.lens
            .get(&local)
            .or_else(|| self.lens.get(&self.root(local)))
            .copied()
    }
}
