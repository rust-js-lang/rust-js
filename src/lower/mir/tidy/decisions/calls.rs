//! What a `match`'s tree whose guard calls does, `[n, ..] if below(*n)`
//! (#221), each way its tests come out, and the chain of its arms that
//! does the same: each call made where the tree makes it or not at all,
//! and what's read after a call what it can't change. Of a tree of pure
//! tests, `decisions` tries the arms in any order; of one that calls, this
//! is what says an arm's test tells.

use crate::js::{self, Expr, ExprKind, Op, UnaryOp};
use crate::lower::mir::hints::Guard;

use super::{Arm, Set, all, alternatives, atom, factored, leaf_tests, pure, same_subject, simplified, within};

/// Ways of the tests' coming out `Model` tries, at most.
const WAYS: usize = 1 << 12;

/// One way the tree's tests come out (`Model`), and what the tree does
/// then: the arm it reaches, and the calls it makes, in order.
struct Outcome {
    way: Way,
    arm: usize,
    calls: Vec<usize>,
}

impl Model<'_> {
    /// What the tree of `arms`' paths does, each way its tests can come
    /// out, of each value tested apart from the others, so what holds of
    /// all of them holds of those a program has: `None` where there are
    /// too many, or one it can't tell.
    pub(super) fn of<'e>(arms: &[Arm], calls: &'e [Expr], guards: &[Guard]) -> Option<Model<'e>> {
        if (0..calls.len()).any(|i| calls[..i].iter().any(|c| c.written_alike(&calls[i]))) {
            return None;
        }
        let mut model = Model {
            subjects: Vec::new(),
            opaque: Vec::new(),
            calls,
            made: (calls.iter())
                .map(|c| guards.iter().find(|g| !c.span.is_none() && within(c.span, g.span)))
                .map(|g| g.and_then(|g| g.tested.first().copied()))
                .collect(),
            outcomes: Vec::new(),
        };
        let paths: Vec<(usize, &Vec<Expr>)> = (arms.iter().enumerate())
            .flat_map(|(i, arm)| arm.made.iter().map(move |path| (i, path)))
            .collect();
        let parts = |test: &'_ Expr| {
            let mut parts = Vec::new();
            leaf_tests(test, js::Span::NONE, &mut parts);
            parts
                .into_iter()
                .map(|(part, span)| (part.clone(), span))
                .collect::<Vec<_>>()
        };
        // Each value tested, and the values of each its tests tell apart.
        let tests: Vec<&Expr> = paths.iter().flat_map(|(_, path)| path.iter()).collect();
        for test in &tests {
            for (part, _) in parts(test) {
                model.add(&part)?;
            }
        }
        // What a guard's `match` keeps: each value one of its tests reads.
        for test in &tests {
            for (part, span) in parts(test) {
                let Some(read) = model.read(&part) else { continue };
                for g in guards
                    .iter()
                    .filter(|g| !span.is_none() && g.tested.iter().any(|t| within(span, *t)))
                {
                    let kept = match read {
                        Read::Subject(j, _) => &mut model.subjects[j].2,
                        Read::Opaque(j, _) => &mut model.opaque[j].1,
                        _ => continue,
                    };
                    kept.extend(g.tested.first().copied());
                }
            }
        }
        // Each value apart, of the runs of values its tests tell apart:
        // a test the chain makes of them, `n < 2` of `n >= 1` and `n === 2`,
        // passes all of one or none.
        for (_, cells, _) in &mut model.subjects {
            *cells = (cells.iter())
                .flat_map(|cell| {
                    let nan = cell.nan.then(|| Set {
                        kind: cell.kind,
                        pieces: Vec::new(),
                        nan: true,
                    });
                    (cell.pieces.iter())
                        .map(|piece| Set {
                            kind: cell.kind,
                            pieces: vec![piece.clone()],
                            nan: false,
                        })
                        .chain(nan)
                        .collect::<Vec<_>>()
                })
                .collect();
        }
        let ways = (model.subjects.iter().map(|s| s.1.len()))
            .try_fold(1usize, |n, cells| n.checked_mul(cells))
            .and_then(|n| n.checked_shl(u32::try_from(model.opaque.len() + calls.len()).ok()?))
            .filter(|&n| n <= WAYS)?;
        for way in 0..ways {
            let mut rest = way;
            let cells: Vec<usize> = (model.subjects.iter())
                .map(|s| {
                    let cell = rest % s.1.len();
                    rest /= s.1.len();
                    cell
                })
                .collect();
            let at = Way { cells, bits: rest };
            // The one path whose tests each pass, tried as the tree tries them.
            let mut reached = None;
            for (arm, path) in &paths {
                let mut trace = Vec::new();
                let mut passes = true;
                for test in path.iter() {
                    if !model.eval(test, &at, &mut trace)? {
                        passes = false;
                        break;
                    }
                }
                if passes {
                    if reached.is_some() {
                        return None;
                    }
                    reached = Some((*arm, trace));
                }
            }
            let (arm, calls) = reached?;
            model.outcomes.push(Outcome { way: at, arm, calls });
        }
        Some(model)
    }

    /// Whether `test`, arm `arm`'s after `steps`, the tests of the arms
    /// before it, does what the tree does each way the chain tries it:
    /// passes where the tree reaches that arm, making the calls the tree
    /// makes, and otherwise fails, having made only calls the tree makes
    /// first.
    fn tells(&self, arm: usize, steps: &[(usize, Expr)], test: &Expr) -> bool {
        self.outcomes.iter().all(|outcome| {
            let mut trace = Vec::new();
            for (_, earlier) in steps {
                match self.eval(earlier, &outcome.way, &mut trace) {
                    Some(true) => return true,
                    Some(false) => {}
                    None => return false,
                }
            }
            match self.eval(test, &outcome.way, &mut trace) {
                Some(true) => outcome.arm == arm && trace == outcome.calls,
                Some(false) => outcome.arm != arm && outcome.calls.starts_with(&trace),
                None => false,
            }
        })
    }

    /// Whether the chain of `steps`, each arm's test tried in turn and
    /// `otherwise`'s arm where none passes, does what the tree does, each
    /// way.
    pub(super) fn kept(&self, steps: &[(usize, Expr)], otherwise: usize) -> bool {
        self.outcomes.iter().all(|outcome| {
            let mut trace = Vec::new();
            let mut arm = otherwise;
            for (i, test) in steps {
                match self.eval(test, &outcome.way, &mut trace) {
                    Some(true) => {
                        arm = *i;
                        break;
                    }
                    Some(false) => {}
                    None => return false,
                }
            }
            arm == outcome.arm && trace == outcome.calls
        })
    }

    /// Arm `arm`'s test of its `paths`, after `steps`, the arms' before
    /// it, with no more than the tree needs: a test that what's tested
    /// before it says, `x !== 1` once `x === 1` failed, or a call's `!odd(b)`
    /// once `odd(b)` was made and failed, goes, as do paths a shorter one
    /// says all of. `None` where its paths as they are don't tell.
    pub(super) fn fewest(&self, arm: usize, steps: &[(usize, Expr)], mut paths: Vec<Vec<Expr>>) -> Option<Expr> {
        // Each tried as `alternatives` says, after what they test first
        // alike, `(1, _) | (_, 1)`'s `a === 1` before `a !== 1 && b === 1`.
        let shared = (0..paths.first().map_or(0, Vec::len))
            .take_while(|&t| (paths.iter()).all(|path| path.len() > t + 1 && path[t].written_alike(&paths[0][t])))
            .count();
        let before: Vec<Expr> = paths.first().map(|path| path[..shared].to_vec()).unwrap_or_default();
        let mut rest: Vec<Vec<Expr>> = paths.iter().map(|path| path[shared..].to_vec()).collect();
        alternatives(&mut rest);
        paths = rest.into_iter().map(|path| [before.clone(), path].concat()).collect();
        // What each path's tests before say, and what the paths before it
        // and the arms' before them do.
        for j in 0..paths.len() {
            let mut t = 0;
            while t < paths[j].len() {
                if self.says(steps, &paths[..j], &paths[j][..t], &paths[j][t]) {
                    paths[j].remove(t);
                } else {
                    t += 1;
                }
            }
        }
        merged(&mut paths);
        if !self.tells(arm, steps, &factored(paths.clone())) {
            return None;
        }
        // Each test that can go, with the paths it then says all of, one
        // at a time, each time from the first, as the paths are fewer.
        while let Some(fewer) = (0..paths.len())
            .flat_map(|j| (0..paths[j].len()).map(move |t| (j, t)))
            .find_map(|(j, t)| {
                let mut fewer = paths.clone();
                fewer[j].remove(t);
                let alone = vec![fewer[j].clone()];
                let others = (0..fewer.len()).filter(|&o| o != j).map(|o| {
                    let mut without = fewer.clone();
                    without.remove(o);
                    without
                });
                [fewer.clone(), alone]
                    .into_iter()
                    .chain(others)
                    .find(|candidate| self.tells(arm, steps, &factored(candidate.clone())))
            })
            .or_else(|| {
                // A path the others say all of.
                (0..paths.len()).filter(|_| paths.len() > 1).find_map(|o| {
                    let mut without = paths.clone();
                    without.remove(o);
                    self.tells(arm, steps, &factored(without.clone())).then_some(without)
                })
            })
        {
            paths = fewer;
        }
        // Each path's tests of one value read as one, where that tells too.
        let read: Option<Vec<Vec<Expr>>> = paths.iter().map(|path| simplified(path)).collect();
        if let Some(read) = read
            && self.tells(arm, steps, &factored(read.clone()))
        {
            paths = read;
        }
        Some(factored(paths))
    }

    /// Whether `test`, after `prefix` of its path passes, its arm's `paths`
    /// before it and the `steps` before that failing, passes each way the
    /// tree's tests come out.
    fn says(&self, steps: &[(usize, Expr)], paths: &[Vec<Expr>], prefix: &[Expr], test: &Expr) -> bool {
        self.outcomes.iter().all(|outcome| {
            let mut trace = Vec::new();
            let earlier = steps
                .iter()
                .map(|(_, t)| t.clone())
                .chain(paths.iter().cloned().map(all));
            for earlier in earlier {
                match self.eval(&earlier, &outcome.way, &mut trace) {
                    Some(true) => return true,
                    Some(false) => {}
                    None => return false,
                }
            }
            for before in prefix {
                match self.eval(before, &outcome.way, &mut trace) {
                    Some(true) => {}
                    Some(false) => return true,
                    None => return false,
                }
            }
            self.eval(test, &outcome.way, &mut trace) == Some(true)
        })
    }
}

/// Paths one after another alike but for one test, which one makes and
/// the other its converse, `a === 1 && x` and `a !== 1 && x`: one, without
/// it, `x`, which each way is tried as before.
fn merged(paths: &mut Vec<Vec<Expr>>) {
    let mut j = 0;
    while j + 1 < paths.len() {
        let (a, b) = (&paths[j], &paths[j + 1]);
        let apart: Vec<usize> = (0..a.len())
            .filter(|&t| t >= b.len() || !a[t].written_alike(&b[t]))
            .collect();
        match apart.as_slice() {
            [t] if a.len() == b.len() && pure(&a[*t]) && a[*t].clone().negated_test().written_alike(&b[*t]) => {
                let t = *t;
                paths[j].remove(t);
                paths.remove(j + 1);
                j = j.saturating_sub(1);
            }
            _ => j += 1,
        }
    }
}

/// What a tree whose guard calls does (#221): each value tested, of one
/// kind, the values its tests tell apart, and the `match`es that keep it;
/// each test of no constant, `x != null` as `x == null`, and those that
/// keep it; each call, and its guard's `match`; and each way the tests
/// come out, what the tree does then. Of a chain of pure tests, any order
/// of its arms does what the tree does; of one that calls, a call is
/// made where the tree makes it, or not at all, and what's read after it
/// is what it can't change.
pub(super) struct Model<'e> {
    subjects: Vec<(Expr, Vec<Set>, Vec<js::Span>)>,
    opaque: Vec<(Expr, Vec<js::Span>)>,
    calls: &'e [Expr],
    made: Vec<Option<js::Span>>,
    outcomes: Vec<Outcome>,
}

/// A part of a test, as `Model` reads it.
#[derive(Clone)]
enum Read {
    Subject(usize, Set),
    /// Whether it's the `!=` of the test.
    Opaque(usize, bool),
    Call(usize),
    Constant(bool),
}

/// One way the tests come out: each value's, the tests of no constant's
/// and the calls', a bit each, in that order.
struct Way {
    cells: Vec<usize>,
    bits: usize,
}

impl Model<'_> {
    /// `part` told of, `None` where it can't be.
    fn add(&mut self, part: &Expr) -> Option<()> {
        match self.read(part) {
            Some(Read::Subject(j, set)) => {
                let cells = std::mem::take(&mut self.subjects[j].1);
                self.subjects[j].1 = (cells.iter())
                    .flat_map(|c| [c.intersect(&set), c.intersect(&set.complement())])
                    .filter(|c| !c.is_empty())
                    .collect();
                Some(())
            }
            Some(_) => Some(()),
            None if !pure(part) => None,
            None => {
                if let Some((subject, set)) = atom(part) {
                    self.subjects.push((subject, vec![Set::all(set.kind)], Vec::new()));
                } else {
                    self.opaque.push((opaque(part).0, Vec::new()));
                }
                self.add(part)
            }
        }
    }

    fn read(&self, part: &Expr) -> Option<Read> {
        if let ExprKind::Bool(b) = part.kind {
            return Some(Read::Constant(b));
        }
        if !pure(part) {
            return self.calls.iter().position(|c| c.written_alike(part)).map(Read::Call);
        }
        if let Some((subject, set)) = atom(part) {
            let j = (self.subjects.iter()).position(|(s, cells, _)| {
                same_subject(s, &subject) && cells.first().is_none_or(|c| c.kind == set.kind)
            })?;
            return Some(Read::Subject(j, set));
        }
        let (test, negated) = opaque(part);
        let j = self.opaque.iter().position(|(t, _)| t.written_alike(&test))?;
        Some(Read::Opaque(j, negated))
    }

    /// Whether `test` passes the way `at`, with what it calls, in order,
    /// in `trace`: `None` where it reads after a call what that call may
    /// change, or can't tell.
    fn eval(&self, test: &Expr, at: &Way, trace: &mut Vec<usize>) -> Option<bool> {
        match &test.kind {
            ExprKind::Binary(Op::And, a, b) => match self.eval(a, at, trace)? {
                true => self.eval(b, at, trace),
                false => Some(false),
            },
            ExprKind::Binary(Op::Or, a, b) => match self.eval(a, at, trace)? {
                true => Some(true),
                false => self.eval(b, at, trace),
            },
            ExprKind::Unary(UnaryOp::Not, a) => self.eval(a, at, trace).map(|b| !b),
            _ => {
                let bit = |k: usize| at.bits >> k & 1 == 1;
                let read = self.read(test)?;
                let kept = match &read {
                    Read::Subject(j, _) => self.subjects[*j].2.as_slice(),
                    Read::Opaque(j, _) => self.opaque[*j].1.as_slice(),
                    _ => &[],
                };
                if matches!(read, Read::Subject(..) | Read::Opaque(..))
                    && !(trace.iter()).all(|&c| self.made[c].is_some_and(|m| kept.contains(&m)))
                {
                    return None;
                }
                match read {
                    Read::Constant(b) => Some(b),
                    Read::Call(k) => {
                        trace.push(k);
                        Some(bit(self.opaque.len() + k))
                    }
                    Read::Opaque(j, negated) => Some(bit(j) != negated),
                    Read::Subject(j, set) => {
                        let cell = &self.subjects[j].1[at.cells[j]];
                        if cell.within(&set) {
                            Some(true)
                        } else if cell.intersect(&set).is_empty() {
                            Some(false)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }
}

/// A test of no constant as `Model` reads it, `x != null` as `x == null`,
/// and whether it's the converse.
fn opaque(part: &Expr) -> (Expr, bool) {
    match &part.kind {
        ExprKind::Binary(op @ (Op::Ne | Op::LooseNe), a, b) => {
            let op = if *op == Op::Ne { Op::Eq } else { Op::LooseEq };
            (Expr::bin(op, (**a).clone(), (**b).clone()), true)
        }
        _ => (part.clone(), false),
    }
}
