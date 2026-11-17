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

impl Comparison {
    /// The comparison that holds where this one doesn't: `a > b` of
    /// `a <= b`, `a != b` of `a == b`.
    pub(super) fn negated(self) -> Option<Comparison> {
        let op = match self.op {
            BinOp::Lt => BinOp::Ge,
            BinOp::Le => BinOp::Gt,
            BinOp::Gt => BinOp::Le,
            BinOp::Ge => BinOp::Lt,
            BinOp::Eq => BinOp::Ne,
            BinOp::Ne => BinOp::Eq,
            _ => return None,
        };
        Some(Comparison { op, ..self })
    }
}

/// What a comparison compares: a local or a constant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Value {
    Local(Local),
    Constant(i128),
    /// A constant that doesn't evaluate, a generic one, `N` of
    /// `[u8; N]`: one of its type's range, at least the one and at most the
    /// other (#1742).
    Range(i128, i128),
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
    /// `x` below a slice's length wherever the slice holds more than `c`
    /// items, `(x, slice, c)`: where a path that knew `x` below it joins
    /// one that knew `x` at most `c`, `best = 0` before a loop and
    /// `best = index` in it (#1814).
    pub(super) below_past: IndexSet<(Local, Local, i128)>,
    pub(super) least: IndexMap<Local, i128>,
    /// Each `str::split` or `rsplit` a local holds as it was made, nothing
    /// having stepped it: what it gives collected is never empty (#1860).
    pub(super) splits: IndexSet<Local>,
    pub(super) empties: IndexMap<Local, Local>,
    /// A `for` loop's range of integers, by the local holding it, a `&mut`
    /// of it, or the `Option` its `next()` gave.
    pub(super) ranges: IndexMap<Local, Ranged>,
    /// `t = x + c` of a constant at least one, by `t`: what `x` is, `c`,
    /// and whether it never wraps. `x + c` again is `t`, wrapped or not,
    /// `i += 1` past `i + 1 < n`; and of one that never wraps, `t < y`
    /// is `x < y` too, `items[i + 1]` under `i + 1 < len`.
    pub(super) sums: IndexMap<Local, (Local, i128, bool)>,
    /// A slice's `get(index)`, by the `Option` it gave or its discriminant:
    /// the index, and the most an index below the slice's length is, which
    /// its `Some` knows it is.
    pub(super) gets: IndexMap<Local, (Local, i128)>,
    /// A slice iterator's `position()` or `rposition()`, by the `Option` it
    /// gave, or the `ControlFlow` a `?` of it branched to: the slice it
    /// stepped, whose length the index is below, and the variant that holds
    /// it, `Some`'s or `Continue`'s (#1945).
    pub(super) positions: IndexMap<Local, (Local, u32)>,
    /// Each slice iterator, `s.iter()`, a `&mut` lent of one or a move of
    /// it: the slice it steps, which a `position()` of it is an index of
    /// (#1945). What the walk steps itself is `ranges`'.
    pub(super) iterates: IndexMap<Local, Local>,
    /// Each local holding a `Vec`, or a reference to one, a `push` made
    /// hold an item at least, nothing lending it `&mut` since (#1700).
    pub(super) pushed: IndexSet<Local>,
    /// Each `&mut` of a `Vec` whole, `&mut _1` or `&mut (*_1)`, by the
    /// local it's held in: what it lends, a `push` of it makes pushed.
    pub(super) lending: IndexMap<Local, Local>,
    /// Each integer local lent `&mut` where the walk forgets it
    /// (`lent_where_borrowed`), until it's assigned again: what the borrow
    /// lends may have become a raw pointer that writes it after, as a read
    /// through it leaves the pointer able to, so no test of it, nor any
    /// relation, tells what it is (#1782).
    pub(super) lent_away: IndexSet<Local>,
    /// Each `Vec` given `&mut` to code that may keep a raw pointer to it
    /// (`called_on_mut`), until it's assigned again: a call that may write,
    /// a drop that runs code or a write through a raw pointer may change
    /// its length, as a cell's, so a test of it holds only until one
    /// (#1966).
    pub(super) escaped: IndexSet<Local>,
    /// Each held guard's cell, by the guard, from its `borrow_mut()` to
    /// its drop: no code but through the guard changes it (`guards`,
    /// #1815).
    pub(super) guarded: IndexMap<Local, Local>,
    /// Each map and key, by their roots, the map holds the key of (`keys`).
    pub(super) keys: IndexSet<(Local, Local)>,
    /// Each `bool` a `contains_key` gave, by the map and the key it asks.
    pub(super) has_keys: IndexMap<Local, (Local, Local)>,
    /// Each two slices as long as each other, the lesser local first
    /// (`tables`): `vec![0.0; terraces.len()]` and `terraces`.
    pub(super) equal_lens: IndexSet<(Local, Local)>,
    /// Each table of rows, by the row each of its rows is as long as.
    pub(super) rows: IndexMap<Local, Local>,
    /// Each half of what's never negative, `h = n >> 1` or `n / 2`, by `h`:
    /// `n`, a slice's length or an integer, of which twice what's at most
    /// `h` is at most it, `left = (index + 1) * 2 - 1` below a heap's
    /// length under `index < length >> 1` (ADR 1168, #1948). The one scaled
    /// form the relations know.
    pub(super) halves: IndexMap<Local, Term>,
    /// Each twice what's below a half of `n`, by the product, `b = 2 *
    /// index` under `index < n >> 1`: at most `n - 2`, so `b + 1` is below
    /// `n`, a heap's `2 * index + 1` (`halves`, ADR 1168).
    pub(super) doubled: IndexMap<Local, Term>,
}

/// A range of integers `next()` alone steps (`stepped_ranges`): what its
/// start is at least, and its end, which never changes while it's known.
/// Each value its `next()` gives is at least the one and below the other.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Ranged {
    pub(super) held: Held,
    pub(super) least: i128,
    pub(super) end: End,
}

/// What a range's values are bounded by.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum End {
    /// `a..b`: each value below `b`.
    Below(Value),
    /// `a..=b`: each value at most `b`.
    AtMost(Value),
    /// A slice's iterator, `s.iter()`: its items, no integer of them.
    Items(Local),
    /// Enumerated, `s.iter().enumerate()`: each index below its length.
    Indexes(Local),
}

impl End {
    /// Whether it's of `local`, which a write of it makes stale.
    pub(super) fn of(self, local: Local) -> bool {
        match self {
            End::Below(Value::Local(l)) | End::AtMost(Value::Local(l)) | End::Items(l) | End::Indexes(l) => l == local,
            End::Below(Value::Constant(_) | Value::Range(..)) | End::AtMost(Value::Constant(_) | Value::Range(..)) => {
                false
            }
        }
    }
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
            below: self.below_joined(other),
            below_past: self.below_past_joined(other),
            least,
            splits: (self.splits.iter())
                .filter(|s| other.splits.contains(*s))
                .copied()
                .collect(),
            empties: both(&self.empties, &other.empties),
            ranges,
            sums: (self.sums.iter())
                .filter(|&(k, v)| other.sums.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect(),
            gets: (self.gets.iter())
                .filter(|&(k, v)| other.gets.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect(),
            positions: (self.positions.iter())
                .filter(|&(k, v)| other.positions.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect(),
            iterates: (self.iterates.iter())
                .filter(|&(k, v)| other.iterates.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect(),
            lent_away: self.lent_away.union(&other.lent_away).copied().collect(),
            escaped: self.escaped.union(&other.escaped).copied().collect(),
            pushed: (self.pushed.iter())
                .filter(|l| other.pushed.contains(*l))
                .copied()
                .collect(),
            lending: both(&self.lending, &other.lending),
            guarded: both(&self.guarded, &other.guarded),
            keys: self.keys.intersection(&other.keys).copied().collect(),
            has_keys: (self.has_keys.iter())
                .filter(|&(k, v)| other.has_keys.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect(),
            equal_lens: self.equal_lens.intersection(&other.equal_lens).copied().collect(),
            rows: both(&self.rows, &other.rows),
            halves: (self.halves.iter())
                .filter(|&(k, v)| other.halves.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect(),
            doubled: (self.doubled.iter())
                .filter(|&(k, v)| other.doubled.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect(),
        }
    }

    /// Each relation of `self` or `other` that holds of the other too: in
    /// both, strictly where both are strict; or in one, and of the other's
    /// intervals, `i = 0` at most a length before a loop, `i < len` past
    /// its back edge, at a loop's head `i <= len`.
    pub(super) fn below_joined(&self, other: &State) -> IndexSet<(Local, Term, bool)> {
        let mut joined = IndexSet::default();
        for (one, two) in [(self, other), (other, self)] {
            for &(x, y, strict) in &one.below {
                let there = match (two.below.contains(&(x, y, true)), two.below.contains(&(x, y, false))) {
                    (true, _) => Some(true),
                    (false, true) => Some(false),
                    _ => two.below_by_intervals(x, y),
                };
                if let Some(there) = there {
                    joined.insert((x, y, strict && there));
                }
            }
        }
        joined
    }

    /// Each guarded relation `below_past` keeps at a join: `x` strictly
    /// below a slice's length on one way and at most `c` on the other, or
    /// guarded so on one way and so or below on the other, the larger `c`.
    pub(super) fn below_past_joined(&self, other: &State) -> IndexSet<(Local, Local, i128)> {
        let mut joined: IndexMap<(Local, Local), i128> = IndexMap::default();
        let mut keep = |x: Local, slice: Local, c: i128| {
            let at = joined.entry((x, slice)).or_insert(c);
            *at = (*at).max(c);
        };
        for (one, two) in [(self, other), (other, self)] {
            let most = |x: Local| two.intervals[x].filter(|i| 0 <= i.lo).map(|i| i.hi);
            for &(x, y, strict) in &one.below {
                if let (Term::Len(slice), true) = (y, strict)
                    && !two.below.contains(&(x, y, true))
                {
                    // Guarded on the other way, its guard; else its most.
                    let guarded = two
                        .below_past
                        .iter()
                        .find(|&&(z, s, _)| (z, s) == (x, slice))
                        .map(|&(_, _, c)| c);
                    if let Some(c) = guarded.or_else(|| most(x)) {
                        keep(x, slice, c);
                    }
                }
            }
            for &(x, slice, c) in &one.below_past {
                if two.below.contains(&(x, Term::Len(slice), true)) {
                    keep(x, slice, c);
                } else if let Some(&(_, _, there)) = two.below_past.iter().find(|&&(y, s, _)| (y, s) == (x, slice)) {
                    keep(x, slice, c.max(there));
                } else if let Some(most) = most(x) {
                    keep(x, slice, c.max(most));
                }
            }
        }
        joined.into_iter().map(|((x, slice), c)| (x, slice, c)).collect()
    }

    /// The least a slice's length is: what's known of it, or one past the
    /// least of what's below it.
    pub(super) fn least_of(&self, slice: Local) -> i128 {
        let below = (self.below.iter())
            .filter(|&&(_, y, _)| y == Term::Len(slice))
            .filter_map(|&(x, _, strict)| Some(self.intervals[x]?.lo + i128::from(strict)))
            .max();
        self.least.get(&slice).copied().unwrap_or(0).max(below.unwrap_or(0))
    }

    /// Whether `x` is below `y`, strictly, as their intervals alone say:
    /// `x`'s most below, or at most, `y`'s least.
    pub(super) fn below_by_intervals(&self, x: Local, y: Term) -> Option<bool> {
        let most = self.intervals[x]?.hi;
        let least = match y {
            Term::Len(slice) => self.least.get(&slice).copied().unwrap_or(0),
            Term::Int(l) => self.intervals[l]?.lo,
        };
        match (most < least, most <= least) {
            (true, _) => Some(true),
            (false, true) => Some(false),
            _ => None,
        }
    }

    /// `local` written: what it held, and each comparison of it, forgotten.
    pub(super) fn forget(&mut self, local: Local) {
        self.lent_away.shift_remove(&local);
        self.escaped.shift_remove(&local);
        self.intervals[local] = None;
        self.comparisons.shift_remove(&local);
        self.comparisons
            .retain(|_, c| c.a != Value::Local(local) && c.b != Value::Local(local));
        self.copies.retain(|&copy, &mut of| copy != local && of != local);
        self.lens.retain(|&holder, &mut of| holder != local && of != local);
        self.below
            .retain(|&(x, y, _)| x != local && y != Term::Len(local) && y != Term::Int(local));
        self.below_past.retain(|&(x, slice, _)| x != local && slice != local);
        self.least.shift_remove(&local);
        self.splits.shift_remove(&local);
        self.empties.retain(|&holder, &mut of| holder != local && of != local);
        self.ranges.shift_remove(&local);
        self.ranges.retain(|_, ranged| !ranged.end.of(local));
        self.sums.retain(|&sum, &mut (of, _, _)| sum != local && of != local);
        self.gets
            .retain(|&gave, &mut (index, _)| gave != local && index != local);
        self.positions
            .retain(|&gave, &mut (slice, _)| gave != local && slice != local);
        self.guarded.retain(|&guard, &mut cell| guard != local && cell != local);
        self.keys.retain(|&(map, key)| map != local && key != local);
        self.has_keys
            .retain(|&gave, &mut (map, key)| gave != local && map != local && key != local);
        self.equal_lens.retain(|&(a, b)| a != local && b != local);
        self.rows.retain(|&table, &mut row| table != local && row != local);
        self.halves
            .retain(|&half, &mut of| half != local && of != Term::Len(local) && of != Term::Int(local));
        self.doubled
            .retain(|&twice, &mut of| twice != local && of != Term::Len(local) && of != Term::Int(local));
    }

    /// Whether a held guard guards `cell`.
    pub(super) fn is_guarded(&self, cell: Local) -> bool {
        self.guarded.values().any(|&c| c == cell)
    }

    /// The comparison a `bool` holds, or what it copies holds: a test held
    /// in a `let`, `truncated = len > LIMIT`, switched on as its copy.
    pub(super) fn comparison_of(&self, local: Local) -> Option<Comparison> {
        (self.comparisons.get(&local))
            .or_else(|| self.comparisons.get(&self.root(local)))
            .copied()
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
        if self.lent_away.contains(&root) {
            return;
        }
        let copies: Vec<Local> = (self.copies.iter())
            .filter(|&(_, &of)| of == root)
            .map(|(&copy, _)| copy)
            .collect();
        for l in [local, root].into_iter().chain(copies) {
            self.intervals[l] = Some(interval);
        }
    }

    /// `x` below `y`, strictly or not: of what `x` copies and of each copy
    /// of that too, as a comparison of one tells of them all.
    pub(super) fn related(&mut self, x: Local, y: Term, strict: bool) {
        if self.lent_away.contains(&self.root(x)) || matches!(y, Term::Int(l) if self.lent_away.contains(&self.root(l)))
        {
            return;
        }
        let root = self.root(x);
        let copies: Vec<Local> = (self.copies.iter())
            .filter(|&(_, &of)| of == root)
            .map(|(&copy, _)| copy)
            .collect();
        for l in [x, root].into_iter().chain(copies) {
            if y != Term::Int(l) && y != Term::Len(l) {
                self.below.insert((l, y, strict));
            }
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
