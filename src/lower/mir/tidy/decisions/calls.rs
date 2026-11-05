//! What a `match`'s tree whose guard calls does, `[n, ..] if below(*n)`
//! (#221), each way its tests come out, and the chain of its arms that
//! does the same: each call made where the tree makes it or not at all,
//! and what's read after a call what it can't change. Of a tree of pure
//! tests, `decisions` tries the arms in any order; of one that calls, this
//! is what says an arm's test tells.

use crate::js::{self, Expr, ExprKind, Op, UnaryOp};
use crate::lower::mir::hints::Guard;

use super::{
    Arm, Set, all, alternatives, any, atom, factored, leaf_tests, pure, same_subject, simplified, within, written_order,
};

/// Ways of the tests' coming out `Model` tries, at most.
const WAYS: usize = 1 << 12;

/// Paths of an arm `Model::fewest` tries in each order, at most.
const ORDERED: usize = 4;

/// One way the tree's tests come out (`Model`), and what the tree does
/// then: the arm it reaches, and the calls it makes, in order.
struct Outcome {
    way: Way,
    arm: usize,
    calls: Vec<Made>,
}

/// A call made: which of the calls it is, and how many of that call were
/// made before it on the way.
type Made = (usize, usize);

impl Model<'_> {
    /// What the tree of `arms`' paths does, each way its tests can come
    /// out, of each value tested apart from the others, so what holds of
    /// all of them holds of those a program has: `None` where there are
    /// too many, or one it can't tell.
    pub(super) fn of<'e>(arms: &[Arm], calls: &'e [Expr], guards: &[Guard]) -> Option<Model<'e>> {
        // Calls alike, one call of the model: a guarded or-pattern's guard,
        // MIR's copy of which each alternative makes, `A | B if on(x)`, of
        // which a path makes one (rd136). One a path makes again, the guard
        // of `(1, _) | (_, 1) if odd(a + b)` where the first alternative's
        // failed, may say something else the second time: each time a path
        // makes it is a call of its own, which the chain makes as often,
        // in the same order (#523).
        let mut unique: Vec<&'e Expr> = Vec::new();
        for call in calls {
            if !unique.iter().any(|c| same_call(c, call)) {
                unique.push(call);
            }
        }
        let mut times = vec![0; unique.len()];
        for path in arms.iter().flat_map(|arm| &arm.made) {
            let mut made = vec![0; unique.len()];
            for test in path {
                let mut parts = Vec::new();
                leaf_tests(test, js::Span::NONE, &mut parts);
                for (part, _) in parts.into_iter().filter(|(part, _)| !pure(part)) {
                    let c = unique.iter().position(|c| same_call(c, part))?;
                    made[c] += 1;
                    times[c] = times[c].max(made[c]);
                }
            }
        }
        let guarded: Vec<Option<&Guard>> = (unique.iter())
            .map(|c| guards.iter().find(|g| !c.span.is_none() && within(c.span, g.span)))
            .collect();
        let made = guarded
            .iter()
            .map(|g| g.and_then(|g| g.tested.first().copied()))
            .collect();
        let within_guard = guarded.iter().map(|g| g.map(|g| g.span)).collect();
        let mut model = Model {
            subjects: Vec::new(),
            opaque: Vec::new(),
            calls: unique,
            times,
            made,
            within_guard,
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
            .and_then(|n| n.checked_shl(u32::try_from(model.opaque.len() + model.times.iter().sum::<usize>()).ok()?))
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
                    if !model.eval(test, &at, &mut trace, false)? {
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
                match self.eval(earlier, &outcome.way, &mut trace, false) {
                    Some(true) => return true,
                    Some(false) => {}
                    None => return false,
                }
            }
            match self.eval(test, &outcome.way, &mut trace, false) {
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
                match self.eval(test, &outcome.way, &mut trace, false) {
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
        alternatives(&mut rest, false);
        paths = rest.into_iter().map(|path| [before.clone(), path].concat()).collect();
        // The paths in the order the tree tries them: as they are, or where
        // a later one is what an earlier one's test that calls says failed,
        // `(1, _) | (_, 1) if odd(a + b)`'s `a === 1 && odd(..)` before `(a
        // !== 1 || !odd(..)) && b === 1 && odd(..)`, that one first (#523).
        let orders = orders(paths.len());
        let (mut paths, mut test) = orders.into_iter().find_map(|order| {
            let mut paths: Vec<Vec<Expr>> = order.iter().map(|&j| paths[j].clone()).collect();
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
            let test = self.told(arm, steps, &paths)?;
            Some((paths, test))
        })?;
        // Each test that can go, with the paths it then says all of, one
        // at a time, each time from the first, as the paths are fewer.
        while let Some((fewer, told)) = (0..paths.len())
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
                    .find_map(|candidate| Some((candidate.clone(), self.told(arm, steps, &candidate)?)))
            })
            .or_else(|| {
                // A path the others say all of.
                (0..paths.len()).filter(|_| paths.len() > 1).find_map(|o| {
                    let mut without = paths.clone();
                    without.remove(o);
                    Some((without.clone(), self.told(arm, steps, &without)?))
                })
            })
        {
            (paths, test) = (fewer, told);
        }
        // Each path's tests of one value read as one, where that tells too.
        let read: Option<Vec<Vec<Expr>>> = paths.iter().map(|path| simplified(path, false)).collect();
        if let Some(read) = read
            && let Some(told) = self.told(arm, steps, &read)
        {
            (paths, test) = (read, told);
        }
        // A run of tests of one tuple's parts in the order its pattern
        // names them, `(2, 2)`'s `a === 2 && b === 2`, where that tells too:
        // no call is between them (`written_order`).
        let mut ordered = paths.clone();
        ordered.iter_mut().for_each(|path| written_order(path));
        if let Some(told) = self.told(arm, steps, &ordered) {
            test = told;
        }
        Some(test)
    }

    /// The test of `paths`, arm `arm`'s after `steps`, that does what the
    /// tree does each way (`tells`): `factored`, what each tests last alike
    /// once, `(a === 1 || b === 1) && on(x)`, or else each path whole, one
    /// after another, where a path makes again a call another made and
    /// failed, `(a === 1 && odd(a + b)) || (b === 1 && odd(a + b))` (#523).
    fn told(&self, arm: usize, steps: &[(usize, Expr)], paths: &[Vec<Expr>]) -> Option<Expr> {
        let factored = factored(paths.to_vec(), &[], false);
        if self.tells(arm, steps, &factored) {
            return Some(factored);
        }
        let whole = any(paths.iter().cloned().map(all).collect());
        self.tells(arm, steps, &whole).then_some(whole)
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
                match self.eval(&earlier, &outcome.way, &mut trace, false) {
                    Some(true) => return true,
                    Some(false) => {}
                    None => return false,
                }
            }
            for before in prefix {
                match self.eval(before, &outcome.way, &mut trace, true) {
                    Some(true) => {}
                    Some(false) => return true,
                    None => return false,
                }
            }
            self.eval(test, &outcome.way, &mut trace, true) == Some(true)
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
    calls: Vec<&'e Expr>,
    /// How many times a path makes each call, at most: a bit of each way
    /// for each time.
    times: Vec<usize>,
    made: Vec<Option<js::Span>>,
    /// Each call's guard, where it's one's: what's read in it after the
    /// call, `[n, ..] if mark(*n) && done.get()`'s `done`, is read there
    /// as the program wrote it, after the call, in the chain as in the tree.
    within_guard: Vec<Option<js::Span>>,
    outcomes: Vec<Outcome>,
}

/// A part of a test, as `Model` reads it.
#[derive(Clone)]
enum Read {
    Subject(usize, Set),
    /// Whether it's the `!=` of the test.
    Opaque(usize, bool),
    /// Whether it's the `!=` of the call's test, `$nextIf(..) == null` of
    /// `$nextIf(..) != null`.
    Call(usize, bool),
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
            let negated = |c: &Expr| opaque(c).1 != opaque(part).1;
            return (self.calls.iter())
                .position(|c| same_call(c, part))
                .map(|k| Read::Call(k, negated(self.calls[k])));
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
    /// change, or can't tell. `again`: a call made before on the way says
    /// what it said then, as what's known of it, not a call made again.
    fn eval(&self, test: &Expr, at: &Way, trace: &mut Vec<Made>, again: bool) -> Option<bool> {
        match &test.kind {
            ExprKind::Binary(Op::And, a, b) => match self.eval(a, at, trace, again)? {
                true => self.eval(b, at, trace, again),
                false => Some(false),
            },
            ExprKind::Binary(Op::Or, a, b) => match self.eval(a, at, trace, again)? {
                true => Some(true),
                false => self.eval(b, at, trace, again),
            },
            ExprKind::Unary(UnaryOp::Not, a) => self.eval(a, at, trace, again).map(|b| !b),
            _ => {
                let bit = |k: usize| at.bits >> k & 1 == 1;
                let read = self.read(test)?;
                let kept = match &read {
                    Read::Subject(j, _) => self.subjects[*j].2.as_slice(),
                    Read::Opaque(j, _) => self.opaque[*j].1.as_slice(),
                    _ => &[],
                };
                let in_its_guard =
                    |c: usize| !test.span.is_none() && self.within_guard[c].is_some_and(|g| within(test.span, g));
                if matches!(read, Read::Subject(..) | Read::Opaque(..))
                    && !(trace.iter()).all(|&(c, _)| self.made[c].is_some_and(|m| kept.contains(&m)) || in_its_guard(c))
                {
                    return None;
                }
                match read {
                    Read::Constant(b) => Some(b),
                    Read::Call(k, negated) => {
                        // Its first time on the way, its second: the call
                        // the tree makes that time.
                        let time = trace.iter().filter(|(c, _)| *c == k).count();
                        let first = self.opaque.len() + self.times[..k].iter().sum::<usize>();
                        if again && time > 0 {
                            return Some(bit(first + time - 1) != negated);
                        }
                        if time >= self.times[k] {
                            return None;
                        }
                        trace.push((k, time));
                        Some(bit(first + time) != negated)
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

/// Whether `a` and `b` are one call of the model, a test and its converse
/// one, `$nextIf(..) != null` and `$nextIf(..) == null` (`opaque`): written
/// alike, a guard's copy MIR makes in each alternative, or the same, a test
/// the walk read twice, one `written_alike` can't compare, of a closure,
/// `$nextIf(chars, (item) => item === "/")` (#523).
fn same_call(a: &Expr, b: &Expr) -> bool {
    let ((a, _), (b, _)) = (opaque(a), opaque(b));
    a.written_alike(&b) || a.kind == b.kind
}

/// A test of no constant as `Model` reads it, `x != null` as `x == null`,
/// `!t` as `t`, a sequence by its last, `((x = b), !odd(x))` as `((x = b),
/// odd(x))` (#523), and whether it's the converse.
fn opaque(part: &Expr) -> (Expr, bool) {
    match &part.kind {
        ExprKind::Unary(UnaryOp::Not, a) => {
            let (test, negated) = opaque(a);
            (test, !negated)
        }
        ExprKind::Sequence(items) if let Some((last, done)) = items.split_last() => {
            let (last, negated) = opaque(last);
            let items = done.iter().cloned().chain([last]).collect();
            (Expr::sequence(items), negated)
        }
        ExprKind::Binary(op @ (Op::Ne | Op::LooseNe), a, b) => {
            let op = if *op == Op::Ne { Op::Eq } else { Op::LooseEq };
            (Expr::bin(op, (**a).clone(), (**b).clone()), true)
        }
        _ => (part.clone(), false),
    }
}

/// The orders of `n` paths a test tries: as they are first, then each
/// other, of as few as a match has; of more, only as they are.
fn orders(n: usize) -> Vec<Vec<usize>> {
    let mut orders = vec![(0..n).collect::<Vec<_>>()];
    if n > ORDERED {
        return orders;
    }
    let mut k = 0;
    while k < orders.len() {
        let order = orders[k].clone();
        for i in 0..n {
            for j in i + 1..n {
                let mut other = order.clone();
                other.swap(i, j);
                if !orders.contains(&other) {
                    orders.push(other);
                }
            }
        }
        k += 1;
    }
    orders
}
