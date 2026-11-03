//! What's known of a body's integers where it is (ADR 0576): each one's
//! range and its bound by a slice's length, and each slice's least length,
//! joined where two paths meet; and of its `OnceCell`s, which are still
//! empty (ADR 0847).

use super::Slice;
use rustc_middle::thir::{ExprId, LocalVarId};
use std::collections::{HashMap, HashSet};

/// The step of a slice's path into a `RefCell`, to the value it holds:
/// `queue.borrow()`'s `Vec`, `([CELL], queue)` (ADR 0881).
pub(super) const CELL: usize = usize::MAX;

/// The step of a bound's path to an integer variable's own value: `i < n`
/// bounds `i` by `n`, `([VALUE], n)`, as by a slice's length. Only of an
/// unsigned integer of 32 bits or fewer, which is never below 0 nor past
/// `u32::MAX`, as a length isn't (`by_value`).
pub(super) const VALUE: usize = usize::MAX - 1;

/// The step of a slice's path to each of its items, of a table whose
/// items are each as long, `vec![vec![0.0; width]; height]`'s rows:
/// `board[r]` of any `r`, `([ITEM], board)` (#370). Only `same` and
/// `lengths` know it, of such a table, and only an index is judged by it.
pub(super) const ITEM: usize = usize::MAX - 2;

/// Whether a bound's `base` is an integer variable's value, not a slice's
/// length.
pub(super) fn by_value(base: &Slice) -> bool {
    base.0 == [VALUE]
}

/// An upper bound of an integer by a slice's length `L`, `⌊(c·L + b) /
/// 2^s⌋`: `L - 1` of an index `i < xs.len()`, `L / 2 - 1` of one below its
/// half; or by another integer's value, `L` that value (`VALUE`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Below {
    pub(super) c: i128,
    pub(super) b: i128,
    pub(super) s: u32,
}

impl Below {
    /// The length itself.
    pub(super) const LENGTH: Below = Below { c: 1, b: 0, s: 0 };

    /// The same bound over its least denominator: `⌊(4·L - 4) / 4⌋` is
    /// `L - 1`. Each bound is made so, or a loop that halves an index, as
    /// `siftUp`'s does, never settles on one.
    pub(super) fn lowest(mut self) -> Below {
        while self.s > 0 && self.c % 2 == 0 && self.b % 2 == 0 {
            self = Below {
                c: self.c / 2,
                b: self.b / 2,
                s: self.s - 1,
            };
        }
        self
    }

    /// This bound and `other` over one denominator.
    fn aligned(self, other: Below) -> (Below, Below) {
        let s = self.s.max(other.s);
        let up = |x: Below| Below {
            c: x.c << (s - x.s),
            b: x.b << (s - x.s),
            s,
        };
        (up(self), up(other))
    }

    pub(super) fn plus(self, other: Below) -> Below {
        let (x, y) = self.aligned(other);
        Below {
            c: x.c + y.c,
            b: x.b + y.b,
            s: x.s,
        }
        .lowest()
    }

    pub(super) fn offset(self, k: i128) -> Below {
        Below {
            b: self.b + (k << self.s),
            ..self
        }
        .lowest()
    }

    pub(super) fn times(self, k: i128) -> Below {
        Below {
            c: self.c * k,
            b: self.b * k,
            ..self
        }
        .lowest()
    }

    /// Whether it's below the length, `≤ L - 1`, whatever `L` is.
    pub(super) fn within(self) -> bool {
        self.c <= 1 << self.s && self.b <= -(1 << self.s)
    }

    /// Whether it's at most the length, `≤ L`, whatever `L` is: where a
    /// range of the slice may end (ADR 0806).
    pub(super) fn up_to(self) -> bool {
        self.c <= 1 << self.s && self.b <= 0
    }

    /// The most it is where the length is at most `length`.
    pub(super) fn most(self, length: i128) -> i128 {
        (self.c * length + self.b) >> self.s
    }

    /// A bound at least either is.
    pub(super) fn join(self, other: Below) -> Below {
        let (x, y) = self.aligned(other);
        Below {
            c: x.c.max(y.c),
            b: x.b.max(y.b),
            s: x.s,
        }
        .lowest()
    }
}

/// Each bound of an integer by a slice's length, or by another integer's
/// value, at most one of each: `i` of `i < prev.len() && i < next.len()`
/// is below either length, so an index of each (rd423). As a set: the
/// order they're found in says nothing, and a loop's rounds settle on the
/// same ones.
#[derive(Clone, Default, Debug)]
pub(super) struct Bounds(Vec<(Slice, Below)>);

impl PartialEq for Bounds {
    fn eq(&self, other: &Bounds) -> bool {
        self.0.len() == other.0.len() && self.0.iter().all(|(s, x)| other.get(s) == Some(*x))
    }
}

impl Bounds {
    /// The one bound, of `slice`.
    pub(super) fn of(slice: Slice, below: Below) -> Bounds {
        Bounds(vec![(slice, below)])
    }

    /// Its bound by `slice`, if it has one.
    pub(super) fn get(&self, slice: &Slice) -> Option<Below> {
        self.0.iter().find(|(s, _)| s == slice).map(|&(_, x)| x)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &(Slice, Below)> {
        self.0.iter()
    }

    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut (Slice, Below)> {
        self.0.iter_mut()
    }

    /// `slice`'s bound `below`, in place of one it had.
    pub(super) fn set(&mut self, slice: Slice, below: Below) {
        match self.0.iter_mut().find(|(s, _)| *s == slice) {
            Some(bound) => bound.1 = below,
            None => self.0.push((slice, below)),
        }
    }

    pub(super) fn retain(&mut self, keep: impl FnMut(&(Slice, Below)) -> bool) {
        self.0.retain(keep);
    }

    pub(super) fn any(&self, holds: impl FnMut(&(Slice, Below)) -> bool) -> bool {
        self.0.iter().any(holds)
    }

    /// Each bound `made` makes of one of these.
    pub(super) fn filter_map(&self, mut made: impl FnMut(&Slice, Below) -> Option<(Slice, Below)>) -> Bounds {
        let mut bounds = Bounds::default();
        for (slice, below) in &self.0 {
            if let Some((slice, below)) = made(slice, *below) {
                bounds.set(slice, below);
            }
        }
        bounds
    }

    /// These, or `other` where there are none.
    pub(super) fn or(self, other: Bounds) -> Bounds {
        match self.is_empty() {
            true => other,
            false => self,
        }
    }

    /// These and `other`'s, each of `other`'s in place of one of the same
    /// slice: each holds.
    pub(super) fn with(mut self, other: Bounds) -> Bounds {
        for (slice, below) in other.0 {
            self.set(slice, below);
        }
        self
    }
}

/// What an integer can be: from `lo` to `hi`, and at most each of `below`
/// by a slice's length or another's value.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Range {
    pub(super) lo: i128,
    pub(super) hi: i128,
    pub(super) below: Bounds,
}

impl Range {
    pub(super) fn of(lo: i128, hi: i128) -> Self {
        Range {
            lo,
            hi,
            below: Bounds::default(),
        }
    }

    pub(super) fn constant(&self) -> Option<i128> {
        (self.lo == self.hi).then_some(self.lo)
    }

    /// Whether a product of this and `other` can be JS's `-0`: one of them
    /// 0, the other below it, `-4 * 0`, which an integer never is.
    pub(super) fn may_make_negative_zero(&self, other: &Range) -> bool {
        let zero = |r: &Range| r.lo <= 0 && 0 <= r.hi;
        zero(self) && other.lo < 0 || zero(other) && self.lo < 0
    }

    /// Its bound by `slice`'s length: its own, or of one without, its
    /// most, `⌊(0·L + hi) / 1⌋`, which it is whatever the length.
    /// Of one bound by another integer's value, which says nothing of a
    /// slice's length, its most too; but by a value, only its own: a range
    /// of no bound joined with one by `n`'s value is of none, or a loop's
    /// counter would never settle.
    pub(super) fn below_of(&self, slice: &Slice) -> Option<Below> {
        match self.below.get(slice) {
            Some(x) => Some(x),
            None if by_value(slice) => None,
            None if self.below.any(|(s, _)| !by_value(s)) => None,
            None => Some(Below { c: 0, b: self.hi, s: 0 }),
        }
    }

    /// A range either is in: of `0` and of an index below `xs.len()`, one
    /// still at most the length; by each slice either is bound by.
    pub(super) fn join(&self, other: &Range) -> Range {
        let mut below = Bounds::default();
        for (slice, _) in self.below.iter().chain(other.below.iter()) {
            if below.get(slice).is_none()
                && let (Some(x), Some(y)) = (self.below_of(slice), other.below_of(slice))
            {
                below.set(slice.clone(), x.join(y));
            }
        }
        Range {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            below,
        }
    }
}

/// What's known where the body is: each integer variable's range, the
/// least length of each slice, and the test each `bool` variable holds.
#[derive(Clone, PartialEq, Default, Debug)]
pub(super) struct Known {
    pub(super) vars: HashMap<LocalVarId, Range>,
    pub(super) lengths: HashMap<Slice, i128>,
    /// Each `bool` variable made of a test that reads without changing
    /// anything, `let truncated = xs.len() > 3`: the test, and the
    /// variables it reads, while neither it nor they change, so a branch
    /// on it shows what the test does where it's taken (ADR 0806).
    pub(super) tests: HashMap<LocalVarId, (ExprId, Vec<LocalVarId>)>,
    /// Each `OnceCell` the body made that's still empty, by the variable
    /// that made it and the fields to it: `payload.io_info` of `let payload
    /// = Rc::new(Payload { io_info: OnceCell::new(), .. })` (ADR 0847).
    pub(super) empty: HashSet<Slice>,
    /// The variables whose cells each variable may reach: its own, of one
    /// that made some, and of what it's made of, a clone of their `Rc`.
    pub(super) reaches: HashMap<LocalVarId, HashSet<LocalVarId>>,
    /// Each slice as long as another's length, or as an integer's value,
    /// while neither changes: `levels` of `let levels = vec![0.0;
    /// terraces.len()]`, `([], terraces)`, so an index below one is below
    /// the other (#362).
    pub(super) same: HashMap<Slice, Slice>,
    /// Each key a map holds, by the map, found as a slice is, and the
    /// variable whose value is the key, while neither changes:
    /// `(([CELL], errors), id)` of `errors.borrow().contains_key(id)`
    /// (`keys`).
    pub(super) keys: HashSet<(Slice, LocalVarId)>,
}

impl Known {
    pub(super) fn join(&self, other: &Known) -> Known {
        let vars = (self.vars.iter())
            .filter_map(|(v, r)| other.vars.get(v).map(|o| (*v, r.join(o))))
            .collect();
        let lengths = (self.lengths.iter())
            .filter_map(|(s, &n)| other.lengths.get(s).map(|&m| (s.clone(), n.min(m))))
            .collect();
        let tests = (self.tests.iter())
            .filter(|&(v, t)| other.tests.get(v) == Some(t))
            .map(|(v, t)| (*v, t.clone()))
            .collect();
        let empty = self.empty.intersection(&other.empty).cloned().collect();
        let keys = self.keys.intersection(&other.keys).cloned().collect();
        let same = (self.same.iter())
            .filter(|&(s, base)| other.same.get(s) == Some(base))
            .map(|(s, base)| (s.clone(), base.clone()))
            .collect();
        let mut reaches = self.reaches.clone();
        for (var, of) in &other.reaches {
            reaches.entry(*var).or_default().extend(of);
        }
        Known {
            vars,
            lengths,
            tests,
            empty,
            reaches,
            same,
            keys,
        }
    }

    /// What's known once `var` may have given what it reaches to code that
    /// could set it: none of those cells is known empty.
    pub(super) fn escaped(&mut self, var: LocalVarId) {
        if let Some(of) = self.reaches.get(&var) {
            self.empty.retain(|(_, made_by)| !of.contains(made_by));
        }
    }

    /// What's known after `slice` grows `by` items, one more of a `push`,
    /// one fewer of a `pop`: a bound `⌊(c·L + b) / 2^s⌋` by its length is
    /// `⌊(c·L' + b - c·by) / 2^s⌋` by the new one, and of a `pop` of none,
    /// which leaves it, larger still. Of a slice whose length isn't known,
    /// at least none, a `push` makes it at least one long (ADR 0746).
    pub(super) fn grown(&mut self, slice: &Slice, by: i128) {
        // A test of the length it had isn't one of the length it has, nor
        // is another as long.
        self.tests.retain(|_, (_, reads)| !reads.contains(&slice.1));
        self.same.retain(|s, base| s != slice && base != slice);
        match self.lengths.get_mut(slice) {
            Some(least) => *least = (*least + by).max(0),
            None if by > 0 => _ = self.lengths.insert(slice.clone(), by),
            None => {}
        }
        for range in self.vars.values_mut() {
            for (at, bound) in range.below.iter_mut() {
                if at == slice {
                    bound.b -= bound.c * by;
                    *bound = bound.lowest();
                }
            }
        }
    }

    /// What a value at least `lo`, and at most `below` by `slice`'s length,
    /// shows of the length: `lo ≤ ⌊(c·L + b) / 2^s⌋` puts `L` at least
    /// `⌈(lo·2^s - b) / c⌉`. So `n == 1` of an `n` that's `xs.len()`, or
    /// `xs.len() >= 2`, puts `xs` at least so long (ADR 0746).
    pub(super) fn at_least(&mut self, (slice, below): &(Slice, Below), lo: i128) {
        if below.c <= 0 || by_value(slice) {
            return;
        }
        let least = ((lo << below.s) - below.b + below.c - 1).div_euclid(below.c);
        if least > 0 {
            let at = self.lengths.entry(slice.clone()).or_insert(0);
            *at = (*at).max(least);
        }
    }

    /// What's known after any code may have changed what a `RefCell`
    /// holds, by another reference to it: nothing of a slice or a map in
    /// one, but `except`'s, whose cell a guard holds (ADR 0881).
    pub(super) fn cells_changed(&mut self, except: Option<&Slice>) {
        let celled = |s: &Slice| s.0.contains(&CELL) && Some(s) != except;
        self.lengths.retain(|s, _| !celled(s));
        self.same.retain(|s, base| !celled(s) && !celled(base));
        self.keys.retain(|(map, _)| !celled(map));
        for range in self.vars.values_mut() {
            range.below.retain(|(s, _)| !celled(s));
        }
    }

    /// Whether anything is known of a slice or a map in a `RefCell`.
    pub(super) fn knows_cells(&self) -> bool {
        self.lengths.keys().any(|s| s.0.contains(&CELL))
            || (self.vars.values()).any(|r| r.below.any(|(s, _)| s.0.contains(&CELL)))
            || self.keys.iter().any(|(map, _)| map.0.contains(&CELL))
    }

    /// What's known after `var` may have changed, as an integer or as what
    /// holds a slice: nothing of it. Of the cells it reaches, its read where
    /// it's changed says (`escaped`).
    pub(super) fn changed(&mut self, var: LocalVarId) {
        self.vars.remove(&var);
        self.lengths.retain(|(_, v), _| *v != var);
        self.same.retain(|(_, v), (_, of)| *v != var && *of != var);
        self.keys.retain(|((_, map), key)| *map != var && *key != var);
        self.tests
            .retain(|held, (_, reads)| *held != var && !reads.contains(&var));
        for range in self.vars.values_mut() {
            range.below.retain(|((_, v), _)| *v != var);
        }
    }
}

impl Known {
    /// What's known after any of `var`'s tables' rows may have changed
    /// length: none is known as long as anything (#370).
    pub(super) fn rows_changed(&mut self, var: LocalVarId) {
        let rows = |(path, v): &Slice| *v == var && path.contains(&ITEM);
        self.same.retain(|s, base| !rows(s) && !rows(base));
        self.lengths.retain(|s, _| !rows(s));
    }

    /// Whether `a` is as long as `b`: the same slice, or one known as long
    /// (#362).
    pub(super) fn same_length(&self, a: &Slice, b: &Slice) -> bool {
        a == b || self.same.get(a) == Some(b) || self.same.get(b) == Some(a)
    }
}

/// Either of two places the body goes on from, or the one it does.
pub(super) fn either(a: Option<Known>, b: Option<Known>) -> Option<Known> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.join(&b)),
        (a, b) => a.or(b),
    }
}
