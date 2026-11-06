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
const WAYS: usize = 1 << 14;

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
        // And a test of what reads only values a `match` keeps, `a % 2 ===
        // 0` of a guard `x % 2 == 0` whose `x` is `a`: kept by each that
        // keeps them all, as no call can change what it reads (E0510, #771).
        for j in 0..model.opaque.len() {
            let mut places = Vec::new();
            if !read_places(&model.opaque[j].0, &mut places) {
                continue;
            }
            let kept = (places.iter()).try_fold(None::<Vec<js::Span>>, |kept, place| {
                let (_, _, of) = model.subjects.iter().find(|(s, ..)| same_subject(s, place))?;
                Some(Some(match kept {
                    None => of.clone(),
                    Some(kept) => kept.into_iter().filter(|m| of.contains(m)).collect(),
                }))
            });
            if let Some(Some(kept)) = kept {
                model.opaque[j].1.extend(kept);
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
        let compiled: Vec<(usize, Vec<Node>)> = (paths.iter())
            .map(|(arm, path)| (*arm, path.iter().map(|test| model.compiled(test)).collect()))
            .collect();
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
            for (arm, path) in &compiled {
                let mut trace = Vec::new();
                let mut passes = true;
                for test in path.iter() {
                    if !model.run(test, &at, &mut trace, false)? {
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
        let steps: Vec<Node> = steps.iter().map(|(_, test)| self.compiled(test)).collect();
        let test = self.compiled(test);
        self.outcomes.iter().all(|outcome| {
            let mut trace = Vec::new();
            for earlier in &steps {
                match self.run(earlier, &outcome.way, &mut trace, false) {
                    Some(true) => return true,
                    Some(false) => {}
                    None => return false,
                }
            }
            match self.run(&test, &outcome.way, &mut trace, false) {
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
        let steps: Vec<(usize, Node)> = steps.iter().map(|(i, test)| (*i, self.compiled(test))).collect();
        self.outcomes.iter().all(|outcome| {
            let mut trace = Vec::new();
            let mut arm = otherwise;
            for (i, test) in &steps {
                match self.run(test, &outcome.way, &mut trace, false) {
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
        let earlier: Vec<Node> = (steps.iter())
            .map(|(_, t)| self.compiled(t))
            .chain(paths.iter().cloned().map(|path| self.compiled(&all(path))))
            .collect();
        let prefix: Vec<Node> = prefix.iter().map(|t| self.compiled(t)).collect();
        let test = self.compiled(test);
        self.outcomes.iter().all(|outcome| {
            let mut trace = Vec::new();
            for earlier in &earlier {
                match self.run(earlier, &outcome.way, &mut trace, false) {
                    Some(true) => return true,
                    Some(false) => {}
                    None => return false,
                }
            }
            for before in &prefix {
                match self.run(before, &outcome.way, &mut trace, true) {
                    Some(true) => {}
                    Some(false) => return true,
                    None => return false,
                }
            }
            self.run(&test, &outcome.way, &mut trace, true) == Some(true)
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

    /// `test` as each way reads it (`Model::run`), what's read of each
    /// part found once, not each way (#771).
    fn compiled(&self, test: &Expr) -> Node {
        match &test.kind {
            ExprKind::Binary(Op::And, a, b) => Node::And(Box::new(self.compiled(a)), Box::new(self.compiled(b))),
            ExprKind::Binary(Op::Or, a, b) => Node::Or(Box::new(self.compiled(a)), Box::new(self.compiled(b))),
            ExprKind::Unary(UnaryOp::Not, a) => Node::Not(Box::new(self.compiled(a))),
            _ => {
                let read = self.read(test);
                // Of each cell of the value it tests, whether it passes, fails
                // or can't tell.
                let cells = match &read {
                    Some(Read::Subject(j, set)) => (self.subjects[*j].1.iter())
                        .map(|cell| match (cell.within(set), cell.intersect(set).is_empty()) {
                            (true, _) => Some(true),
                            (false, true) => Some(false),
                            (false, false) => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                Node::Part {
                    read,
                    span: test.span,
                    cells,
                }
            }
        }
    }

    /// Whether `test`, `compiled`, passes the way `at`, with what it calls,
    /// in order, in `trace`: `None` where it reads after a call what that call may
    /// change, or can't tell. `again`: a call made before on the way says
    /// what it said then, as what's known of it, not a call made again.
    fn run(&self, test: &Node, at: &Way, trace: &mut Vec<Made>, again: bool) -> Option<bool> {
        match test {
            Node::And(a, b) => match self.run(a, at, trace, again)? {
                true => self.run(b, at, trace, again),
                false => Some(false),
            },
            Node::Or(a, b) => match self.run(a, at, trace, again)? {
                true => Some(true),
                false => self.run(b, at, trace, again),
            },
            Node::Not(a) => self.run(a, at, trace, again).map(|b| !b),
            Node::Part { read, span, cells } => {
                let bit = |k: usize| at.bits >> k & 1 == 1;
                let read = read.as_ref()?;
                let kept = match read {
                    Read::Subject(j, _) => self.subjects[*j].2.as_slice(),
                    Read::Opaque(j, _) => self.opaque[*j].1.as_slice(),
                    _ => &[],
                };
                let in_its_guard = |c: usize| !span.is_none() && self.within_guard[c].is_some_and(|g| within(*span, g));
                // A test of constants alone, `0 % 2 === 0` of a guard
                // `0 % 2 == 0`, reads no place a call could change (#771).
                let constant = match read {
                    Read::Opaque(j, _) => reads_no_place(&self.opaque[*j].0),
                    _ => false,
                };
                if matches!(read, Read::Subject(..) | Read::Opaque(..))
                    && !constant
                    && !(trace.iter()).all(|&(c, _)| self.made[c].is_some_and(|m| kept.contains(&m)) || in_its_guard(c))
                {
                    return None;
                }
                match *read {
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
                    Read::Subject(j, _) => cells[at.cells[j]],
                }
            }
        }
    }
}

/// A test as `Model::run` reads it each way: its `&&`s, `||`s and `!`s,
/// and of each part, what's read of it, where it's written and, of a
/// value's test, whether each cell of the value passes it.
enum Node {
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
    Not(Box<Node>),
    Part {
        read: Option<Read>,
        span: js::Span,
        cells: Vec<Option<bool>>,
    },
}

/// Whether `test` reads constants alone, no place (`read_places`).
fn reads_no_place(test: &Expr) -> bool {
    let mut places = Vec::new();
    read_places(test, &mut places) && places.is_empty()
}

/// The places `test` reads, each variable, property or element, `a` of `a
/// % 2 === 0`, into `out`; `false` where it reads what's neither a place
/// nor a constant.
fn read_places<'e>(test: &'e Expr, out: &mut Vec<&'e Expr>) -> bool {
    match &test.kind {
        ExprKind::Var(_) | ExprKind::Member(..) | ExprKind::Index(..) => {
            out.push(test);
            true
        }
        ExprKind::Num(_) | ExprKind::Bool(_) | ExprKind::Str(_) | ExprKind::Null | ExprKind::Undefined => true,
        ExprKind::Binary(_, a, b) => read_places(a, out) && read_places(b, out),
        ExprKind::Unary(_, a) => read_places(a, out),
        _ => false,
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
