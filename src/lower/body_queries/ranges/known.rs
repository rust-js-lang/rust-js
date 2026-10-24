//! What's known of a body's integers where it is (ADR 0576): each one's
//! range and its bound by a slice's length, and each slice's least length,
//! joined where two paths meet.

use super::Slice;
use rustc_middle::thir::{ExprId, LocalVarId};
use std::collections::HashMap;

/// An upper bound of an integer by a slice's length `L`, `⌊(c·L + b) /
/// 2^s⌋`: `L - 1` of an index `i < xs.len()`, `L / 2 - 1` of one below its
/// half.
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

/// What an integer can be: from `lo` to `hi`, and at most `below` by a
/// slice's length.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Range {
    pub(super) lo: i128,
    pub(super) hi: i128,
    pub(super) below: Option<(Slice, Below)>,
}

impl Range {
    pub(super) fn of(lo: i128, hi: i128) -> Self {
        Range { lo, hi, below: None }
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
    pub(super) fn below_of(&self, slice: &Slice) -> Option<Below> {
        match &self.below {
            Some((s, x)) => (s == slice).then_some(*x),
            None => Some(Below { c: 0, b: self.hi, s: 0 }),
        }
    }

    /// A range either is in: of `0` and of an index below `xs.len()`, one
    /// still at most the length.
    pub(super) fn join(&self, other: &Range) -> Range {
        let slice = (self.below.as_ref()).or(other.below.as_ref()).map(|(s, _)| s);
        Range {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            below: slice.and_then(|slice| Some((slice.clone(), self.below_of(slice)?.join(other.below_of(slice)?)))),
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
        Known { vars, lengths, tests }
    }

    /// What's known after `slice` grows `by` items, one more of a `push`,
    /// one fewer of a `pop`: a bound `⌊(c·L + b) / 2^s⌋` by its length is
    /// `⌊(c·L' + b - c·by) / 2^s⌋` by the new one, and of a `pop` of none,
    /// which leaves it, larger still. Of a slice whose length isn't known,
    /// at least none, a `push` makes it at least one long (ADR 0746).
    pub(super) fn grown(&mut self, slice: &Slice, by: i128) {
        // A test of the length it had isn't one of the length it has.
        self.tests.retain(|_, (_, reads)| !reads.contains(&slice.1));
        match self.lengths.get_mut(slice) {
            Some(least) => *least = (*least + by).max(0),
            None if by > 0 => _ = self.lengths.insert(slice.clone(), by),
            None => {}
        }
        for range in self.vars.values_mut() {
            if let Some((at, bound)) = &mut range.below
                && at == slice
            {
                bound.b -= bound.c * by;
                *bound = bound.lowest();
            }
        }
    }

    /// What a value at least `lo`, and at most `below` by `slice`'s length,
    /// shows of the length: `lo ≤ ⌊(c·L + b) / 2^s⌋` puts `L` at least
    /// `⌈(lo·2^s - b) / c⌉`. So `n == 1` of an `n` that's `xs.len()`, or
    /// `xs.len() >= 2`, puts `xs` at least so long (ADR 0746).
    pub(super) fn at_least(&mut self, (slice, below): &(Slice, Below), lo: i128) {
        if below.c <= 0 {
            return;
        }
        let least = ((lo << below.s) - below.b + below.c - 1).div_euclid(below.c);
        if least > 0 {
            let at = self.lengths.entry(slice.clone()).or_insert(0);
            *at = (*at).max(least);
        }
    }

    /// What's known after `var` may have changed, as an integer or as what
    /// holds a slice: nothing of it.
    pub(super) fn changed(&mut self, var: LocalVarId) {
        self.vars.remove(&var);
        self.lengths.retain(|(_, v), _| *v != var);
        self.tests
            .retain(|held, (_, reads)| *held != var && !reads.contains(&var));
        for range in self.vars.values_mut() {
            if range.below.as_ref().is_some_and(|((_, v), _)| *v == var) {
                range.below = None;
            }
        }
    }
}

/// Either of two places the body goes on from, or the one it does.
pub(super) fn either(a: Option<Known>, b: Option<Known>) -> Option<Known> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.join(&b)),
        (a, b) => a.or(b),
    }
}
