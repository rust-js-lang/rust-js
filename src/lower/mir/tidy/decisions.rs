//! A match MIR tests as a tree, an arm more than one of its leaves reaches,
//! `'0'..='9' | '.'`, through a labeled block each leaf breaks out of: an
//! `if` chain of the arms (ADR 0364). The tree's tests read what nothing
//! changes, so an arm's test is what holds on the paths to it; the paths
//! are apart, so the arms are too, in any order, and the one whose test is
//! longest is the chain's `else`.

use std::cmp::Ordering;

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp};

use super::{ends, first_written, names_break};

/// More leaves than a match a person writes has: a tree of tests that
/// aren't one.
const LEAVES: usize = 64;

/// Statements an arm may share with another, copied into each: the jumps
/// that end them, `return w;`, as a person writes each arm's.
const SHARED: usize = 3;

/// Each block MIR made in `stmts` that a match's tree breaks out of,
/// `bb9: { .. }`: the `if` chain of its arms, and what follows the block,
/// where one arm alone goes on to it, that arm's. Of a body `loops` has
/// made what it can of MIR's loops in, whose labeled `break`s it reads.
pub(super) fn decisions(stmts: &mut Vec<Stmt>) {
    let mut i = stmts.len();
    while i > 0 {
        i -= 1;
        let StmtKind::Labeled(label, body) = &stmts[i].kind else {
            continue;
        };
        if !crate::lower::mir::is_block_label(label) {
            continue;
        }
        let Some((chain, takes_rest)) = chain(label, body, &stmts[i + 1..]) else {
            continue;
        };
        if takes_rest {
            stmts.truncate(i);
            stmts.extend(chain);
        } else {
            stmts.splice(i..i + 1, chain);
        }
    }
}

/// A leaf of the tree: the tests that hold on the way to it, in the order
/// they're made, what it does then, and whether that goes on past the
/// block.
struct Leaf<'s> {
    tests: Vec<Expr>,
    body: Vec<&'s Stmt>,
    out: bool,
    /// Whether it ends with a loop whose own `break`s leave the block.
    leaves_loop: bool,
}

/// The arms of the tree in `body`, `after` it in its list: the statements
/// in its place, and whether they take the place of `after` too.
fn chain(label: &str, body: &[Stmt], after: &[Stmt]) -> Option<(Vec<Stmt>, bool)> {
    // A block a `return` of what it makes follows, `?`'s: its arms each
    // return, as `returned_in_branches` writes them.
    if let Some(StmtKind::Return(Some(value))) = after.first().map(|s| &s.kind)
        && matches!(value.kind, ExprKind::Var(_))
    {
        return None;
    }
    // The block's own variables, declared first: declared before the chain;
    // the body's names are its own while it's tidied.
    let declared = body
        .iter()
        .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
        .count();
    let (decls, tree) = body.split_at(declared);
    let mut leaves = Vec::new();
    let mut tested = Vec::new();
    walk(vec![tree], Vec::new(), label, &mut leaves, &mut tested)?;
    // A path no value takes, `c < "0" && c >= "A"`, goes.
    let leaves: Vec<(Vec<Expr>, Leaf)> = (leaves.into_iter())
        .filter_map(|leaf| Some((simplified(&leaf.tests)?, leaf)))
        .collect();
    // Leaves that do one thing, one arm.
    let mut arms: Vec<Arm> = Vec::new();
    for (tests, leaf) in leaves {
        let alike = |arm: &&mut Arm| {
            arm.out == leaf.out && arm.leaves_loop == leaf.leaves_loop && same_body(&arm.body, &leaf.body)
        };
        match arms.iter_mut().find(alike) {
            Some(arm) => arm.tests.push(tests),
            None => arms.push(Arm {
                tests: vec![tests],
                body: leaf.body,
                out: leaf.out,
                leaves_loop: leaf.leaves_loop,
            }),
        }
    }
    // What more than one arm does, copied into each: a jump, or none.
    let shared: Vec<&Stmt> = (arms.iter().enumerate())
        .flat_map(|(i, arm)| arm.body.iter().map(move |s| (i, *s)))
        .filter(|(i, s)| {
            arms[..*i]
                .iter()
                .any(|other| other.body.iter().any(|o| std::ptr::eq(*o, *s)))
        })
        .map(|(_, s)| s)
        .collect();
    if shared.len() > SHARED || !shared.iter().all(|s| ends(s) && size(s) == 1) {
        return None;
    }
    let mut bodies: Vec<Vec<Stmt>> = arms
        .iter()
        .map(|arm| arm.body.iter().map(|s| (*s).clone()).collect())
        .collect();
    for (arm, body) in arms.iter().zip(&mut bodies) {
        if arm.leaves_loop
            && let Some(StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }) =
                body.last_mut().map(|s| &mut s.kind)
        {
            unlabel_breaks(body, label);
        }
    }
    // One arm alone going on past the block, the others ending: what follows
    // the block is that arm's.
    let going_on: Vec<usize> = (0..arms.len()).filter(|&i| arms[i].out).collect();
    let takes_rest = going_on.len() == 1 && !after.is_empty();
    if let [only] = going_on.as_slice()
        && takes_rest
    {
        bodies[*only].extend(after.iter().cloned());
    }
    // The `else`: an arm that does nothing, or the one whose test is longest.
    let empty: Vec<usize> = (0..arms.len()).filter(|&i| bodies[i].is_empty()).collect();
    let otherwise = match empty.as_slice() {
        [only] => *only,
        _ => (0..arms.len()).max_by_key(|&i| arms[i].tests.iter().flatten().map(atoms).sum::<usize>())?,
    };
    // The others in the order they're written: where the test that decides
    // each path to them, its last, or what they do first is.
    let written = |i: usize| {
        let tests = arms[i].tests.iter().filter_map(|tests| written_at(tests.last()?));
        tests.chain(first_written(&bodies[i])).min().unwrap_or(u32::MAX)
    };
    let mut order: Vec<usize> = (0..arms.len()).filter(|&i| i != otherwise).collect();
    order.sort_by_key(|&i| (written(i), i));
    let mut chain: Option<Vec<Stmt>> = match std::mem::take(&mut bodies[otherwise]) {
        body if body.is_empty() => None,
        body => Some(body),
    };
    let mut tests_made = 0;
    for &i in order.iter().rev() {
        // Its patterns in the order they're written, `'0'..='9' | '.'`; one
        // whose test is where the whole `|` is, after those it holds.
        let mut paths = arms[i].tests.clone();
        let spans: Vec<js::Span> = paths.iter().filter_map(|t| t.last()).map(|t| t.span).collect();
        paths.sort_by_key(|tests| {
            let span = tests.last().map(|t| t.span).unwrap_or_default();
            let holds = spans.iter().any(|s| *s != span && span.lo <= s.lo && s.hi <= span.hi);
            (holds, tests.last().and_then(written_at).unwrap_or(u32::MAX))
        });
        let test = any(paths.into_iter().map(all).collect());
        tests_made += atoms(&test);
        let then = std::mem::take(&mut bodies[i]);
        let span = then.first().map(|s| s.span).unwrap_or_default();
        chain = Some(vec![StmtKind::If(test, then, chain).at(span)]);
    }
    // Tests no longer than the tree's, or the tree reads better.
    let written: usize = tested.iter().map(|t| atoms(t)).sum();
    if tests_made > written {
        return None;
    }
    let mut stmts: Vec<Stmt> = decls.to_vec();
    stmts.extend(chain.unwrap_or_default());
    Some((stmts, takes_rest))
}

/// Leaves that do one thing, and the tests of each path to them.
struct Arm<'s> {
    tests: Vec<Vec<Expr>>,
    body: Vec<&'s Stmt>,
    out: bool,
    leaves_loop: bool,
}

/// The leaves of `frames`, the rest of each list the walk is in, innermost
/// last, given that `tests` hold: `None` where one isn't a tree's leaf.
fn walk<'s>(
    mut frames: Vec<&'s [Stmt]>,
    tests: Vec<Expr>,
    label: &str,
    leaves: &mut Vec<Leaf<'s>>,
    tested: &mut Vec<&'s Expr>,
) -> Option<()> {
    if leaves.len() > LEAVES {
        return None;
    }
    while frames.last().is_some_and(|f| f.is_empty()) {
        frames.pop();
    }
    let Some(frame) = frames.last_mut() else {
        // Past the block's end, as its `break` goes.
        leaves.push(Leaf {
            tests,
            body: Vec::new(),
            out: true,
            leaves_loop: false,
        });
        return Some(());
    };
    let current: &'s [Stmt] = frame;
    let (stmt, rest) = current.split_first()?;
    match &stmt.kind {
        StmtKind::If(test, then, otherwise) if pure(test) => {
            *frame = rest;
            if !tested.iter().any(|t| std::ptr::eq(*t, test)) {
                tested.push(test);
            }
            let mut yes = frames.clone();
            yes.push(then);
            walk(yes, [tests.clone(), vec![test.clone()]].concat(), label, leaves, tested)?;
            let mut no = frames;
            if let Some(otherwise) = otherwise {
                no.push(otherwise);
            }
            walk(
                no,
                [tests, vec![test.clone().negated_test()]].concat(),
                label,
                leaves,
                tested,
            )
        }
        StmtKind::Break(Some(l)) if l == label => {
            leaves.push(Leaf {
                tests,
                body: Vec::new(),
                out: true,
                leaves_loop: false,
            });
            Some(())
        }
        _ => {
            // What's done from here on, to what ends it.
            let mut body: Vec<&Stmt> = frames.iter().rev().flat_map(|f| f.iter()).collect();
            if let Some(end) = body.iter().position(|s| ends(s)) {
                body.truncate(end + 1);
            }
            let out = match body.last().map(|s| &s.kind) {
                Some(StmtKind::Break(Some(l))) if l == label => {
                    body.pop();
                    true
                }
                Some(_) => !ends(body[body.len() - 1]),
                None => true,
            };
            // A `break` of the block can't be the chain's, but in a loop it
            // ends with, the loop's own, which leaves to where the block does.
            let breaks = |s: &Stmt| names_break(std::slice::from_ref(s), label);
            let (out, leaves_loop) = match body.split_last() {
                Some((last, before)) if !before.iter().any(|s| breaks(s)) && breaks(last) => {
                    let (StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }) =
                        &last.kind
                    else {
                        return None;
                    };
                    if !loop_breaks(body, label) {
                        return None;
                    }
                    (true, true)
                }
                _ if body.iter().any(|s| breaks(s)) => return None,
                _ => (out, false),
            };
            leaves.push(Leaf {
                tests,
                body,
                out,
                leaves_loop,
            });
            Some(())
        }
    }
}

/// Whether each `break` of `label` in a loop's body `stmts` is in no loop
/// of its own: the loop's, unlabeled.
fn loop_breaks(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().all(|stmt| match &stmt.kind {
        StmtKind::If(_, a, b) => loop_breaks(a, label) && b.as_deref().is_none_or(|b| loop_breaks(b, label)),
        StmtKind::Labeled(_, body) => loop_breaks(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => loop_breaks(a, label) && loop_breaks(b, label),
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            !names_break(body, label)
        }
        _ => true,
    })
}

/// Each `break` of `label` `loop_breaks` found: `break`.
fn unlabel_breaks(stmts: &mut [Stmt], label: &str) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Break(l) if l.as_deref() == Some(label) => *l = None,
            StmtKind::If(_, a, b) => {
                unlabel_breaks(a, label);
                if let Some(b) = b {
                    unlabel_breaks(b, label);
                }
            }
            StmtKind::Labeled(_, body) => unlabel_breaks(body, label),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                unlabel_breaks(a, label);
                unlabel_breaks(b, label);
            }
            _ => {}
        }
    }
}

/// Whether reading `e` does nothing: variables, their properties, and
/// what's made of them without a call.
fn pure(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Num(_)
        | ExprKind::BigInt(_)
        | ExprKind::BigUint(_)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Undefined
        | ExprKind::Null
        | ExprKind::Var(_) => true,
        ExprKind::Member(object, _) => pure(object),
        ExprKind::Index(object, index) => pure(object) && pure(index),
        ExprKind::Unary(_, a) => pure(a),
        ExprKind::Binary(_, a, b) => pure(a) && pure(b),
        ExprKind::Cond(a, b, c) => pure(a) && pure(b) && pure(c),
        _ => false,
    }
}

/// How many statements `stmt` is, with those inside it.
fn size(stmt: &Stmt) -> usize {
    let list = |stmts: &[Stmt]| stmts.iter().map(size).sum::<usize>();
    1 + match &stmt.kind {
        StmtKind::If(_, a, b) => list(a) + b.as_deref().map_or(0, list),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => list(body),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => list(a) + list(b),
        _ => 0,
    }
}

/// Whether `a` and `b` do the same: the same statements, or ones written
/// alike.
fn same_body(a: &[&Stmt], b: &[&Stmt]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| std::ptr::eq(*x, *y) || same_stmt(x, y))
}

/// Whether `a` and `b` do the same: written alike, each part.
pub(super) fn same_stmt(a: &Stmt, b: &Stmt) -> bool {
    match (&a.kind, &b.kind) {
        (StmtKind::Assign(t, v), StmtKind::Assign(u, w)) => t.written_alike(u) && v.written_alike(w),
        (StmtKind::Expr(x), StmtKind::Expr(y)) | (StmtKind::Throw(x), StmtKind::Throw(y)) => x.written_alike(y),
        (StmtKind::Return(x), StmtKind::Return(y)) => match (x, y) {
            (Some(x), Some(y)) => x.written_alike(y),
            (None, None) => true,
            _ => false,
        },
        (StmtKind::Break(x), StmtKind::Break(y)) | (StmtKind::Continue(x), StmtKind::Continue(y)) => x == y,
        _ => false,
    }
}

/// `a && b && ..`, of none `true`.
fn all(tests: Vec<Expr>) -> Expr {
    tests
        .into_iter()
        .reduce(|a, b| Expr::bin(Op::And, a, b))
        .unwrap_or_else(|| Expr::bool(true))
}

/// `a || b || ..`.
fn any(tests: Vec<Expr>) -> Expr {
    tests
        .into_iter()
        .reduce(|a, b| Expr::bin(Op::Or, a, b))
        .unwrap_or_else(|| Expr::bool(false))
}

/// Where the first test `e` is made of is written.
fn written_at(e: &Expr) -> Option<u32> {
    let here = (!e.span.is_none()).then_some(e.span.lo);
    let within = match &e.kind {
        ExprKind::Binary(_, a, b) => written_at(a).into_iter().chain(written_at(b)).min(),
        ExprKind::Unary(_, a) => written_at(a),
        _ => None,
    };
    here.into_iter().chain(within).min()
}

/// How many tests `e` is made of.
fn atoms(e: &Expr) -> usize {
    match &e.kind {
        ExprKind::Binary(Op::And | Op::Or, a, b) => atoms(a) + atoms(b),
        ExprKind::Unary(UnaryOp::Not, a) => atoms(a),
        _ => 1,
    }
}

/// `tests`, each read where it's made, without what those before it in a
/// run of tests of one value already say: `None` where no value passes
/// them all.
fn simplified(tests: &[Expr]) -> Option<Vec<Expr>> {
    let mut flat = Vec::new();
    for test in tests {
        conjuncts(test.clone(), &mut flat);
    }
    let mut out = Vec::new();
    // What holds of each value tested, so far.
    let mut known: Vec<(Expr, Set)> = Vec::new();
    let mut i = 0;
    while i < flat.len() {
        let Some((subject, first)) = atom(&flat[i]) else {
            out.push(flat[i].clone());
            i += 1;
            continue;
        };
        // The run of tests of this value, read one after another.
        let mut run = first.clone();
        let mut sets = vec![first];
        let mut j = i + 1;
        while let Some((next, set)) = flat.get(j).and_then(atom)
            && same_subject(&next, &subject)
            && set.kind == run.kind
        {
            run = run.intersect(&set);
            sets.push(set);
            j += 1;
        }
        let at = known
            .iter()
            .position(|(s, set)| same_subject(s, &subject) && set.kind == run.kind);
        let before = at.map_or_else(|| Set::all(run.kind), |at| known[at].1.clone());
        let now = before.intersect(&run);
        if now.is_empty() {
            return None;
        }
        // As one of the run's own tests says it, or as it reads best.
        match (i..j).find(|&k| before.intersect(&sets[k - i]) == now) {
            Some(k) => out.push(flat[k].clone()),
            None => out.push(Expr {
                span: flat[j - 1].span,
                ..render(&subject, &now)
            }),
        }
        match at {
            Some(at) => known[at].1 = now,
            None => known.push((subject, now)),
        }
        i = j;
    }
    Some(out)
}

/// `test`'s parts that must all hold, `a && b`'s, into `out`.
fn conjuncts(test: Expr, out: &mut Vec<Expr>) {
    match test.kind {
        ExprKind::Binary(Op::And, a, b) => {
            conjuncts(*a, out);
            conjuncts(*b, out);
        }
        kind => out.push(Expr { kind, ..test }),
    }
}

/// The value `e` tests against constants, and the values that pass: of
/// `c >= "a" && c <= "z"`, `c` and `"a"..="z"`.
fn atom(e: &Expr) -> Option<(Expr, Set)> {
    match &e.kind {
        ExprKind::Unary(UnaryOp::Not, a) => atom(a).map(|(s, set)| (s, set.complement())),
        ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => {
            let ((s, x), (t, y)) = (atom(a)?, atom(b)?);
            (same_subject(&s, &t) && x.kind == y.kind).then(|| {
                let set = if *op == Op::And { x.intersect(&y) } else { x.union(&y) };
                (s, set)
            })
        }
        ExprKind::Binary(op @ (Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge), a, b) => {
            let (subject, value, op) = match (Value::of(a), Value::of(b)) {
                (None, Some(value)) => (&**a, value, *op),
                (Some(value), None) => (&**b, value, flipped(*op)),
                _ => return None,
            };
            if !is_subject(subject) {
                return None;
            }
            let kind = value.kind();
            let point = Interval {
                lo: Some((value.clone(), true)),
                hi: Some((value.clone(), true)),
            };
            let pieces = match op {
                Op::Eq | Op::Ne => vec![point],
                Op::Lt | Op::Le => vec![Interval {
                    lo: None,
                    hi: Some((value, op == Op::Le)),
                }],
                _ => vec![Interval {
                    lo: Some((value, op == Op::Ge)),
                    hi: None,
                }],
            };
            let set = Set {
                kind,
                pieces,
                nan: false,
            };
            Some((subject.clone(), if op == Op::Ne { set.complement() } else { set }))
        }
        _ => None,
    }
}

/// `a < b` as `b > a`.
fn flipped(op: Op) -> Op {
    match op {
        Op::Lt => Op::Gt,
        Op::Le => Op::Ge,
        Op::Gt => Op::Lt,
        Op::Ge => Op::Le,
        op => op,
    }
}

/// A value a test reads the same each time: a variable, a property or an
/// item of one, or `typeof` one.
fn is_subject(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Var(_) => true,
        ExprKind::Member(object, _) => is_subject(object),
        ExprKind::Index(object, index) => is_subject(object) && Value::of(index).is_some(),
        ExprKind::Unary(UnaryOp::Typeof, of) => is_subject(of),
        _ => false,
    }
}

fn same_subject(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Unary(UnaryOp::Typeof, x), ExprKind::Unary(UnaryOp::Typeof, y)) => same_subject(x, y),
        _ => a.written_alike(b),
    }
}

/// `subject` in `set`, as a person writes it: the values that pass, or
/// those that don't, whichever is shorter and says the same of `NaN`.
fn render(subject: &Expr, set: &Set) -> Expr {
    let other = set.complement();
    // `x !== a && x !== b`: all but some, and `NaN` if a number.
    let points = other.pieces.iter().all(Interval::is_point) && (set.nan || set.kind != Kind::Num);
    if points {
        return all(other
            .pieces
            .iter()
            .map(|p| test(subject, Op::Ne, &p.lo.as_ref().expect("a point").0))
            .collect());
    }
    let passing = any(set.pieces.iter().map(|p| p.render(subject)).collect());
    // A number's comparisons are false of `NaN`; one where it passes is
    // `!` of those of the others.
    if set.nan {
        return any(other.pieces.iter().map(|p| p.render(subject)).collect()).negated_test();
    }
    let failing = || match set.kind {
        Kind::Num => None,
        _ => Some(all(other.pieces.iter().map(|p| p.render_outside(subject)).collect())),
    };
    match failing() {
        Some(failing) if atoms(&failing) < atoms(&passing) => failing,
        _ => passing,
    }
}

fn test(subject: &Expr, op: Op, value: &Value) -> Expr {
    Expr::bin(op, subject.clone(), value.expr.clone())
}

/// What a test compares to: a string, a number, or a `BigInt`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Str,
    Num,
    Big,
}

/// A constant tested against, and how it's written.
#[derive(Clone)]
struct Value {
    expr: Expr,
    of: Lit,
}

#[derive(Clone, PartialEq)]
enum Lit {
    /// As JS orders strings, by UTF-16 code unit.
    Str(Vec<u16>),
    Num(f64),
    Big(i128),
}

impl Value {
    fn of(e: &Expr) -> Option<Value> {
        let of = match &e.kind {
            ExprKind::Str(s) => Lit::Str(s.encode_utf16().collect()),
            ExprKind::Num(n) if !n.is_nan() => Lit::Num(*n),
            ExprKind::BigInt(n) => Lit::Big(*n),
            ExprKind::BigUint(n) => Lit::Big(i128::try_from(*n).ok()?),
            ExprKind::Unary(UnaryOp::Neg, inner) => match Value::of(inner)?.of {
                Lit::Num(n) => Lit::Num(-n),
                Lit::Big(n) => Lit::Big(n.checked_neg()?),
                Lit::Str(_) => return None,
            },
            _ => return None,
        };
        Some(Value { expr: e.clone(), of })
    }

    fn kind(&self) -> Kind {
        match self.of {
            Lit::Str(_) => Kind::Str,
            Lit::Num(_) => Kind::Num,
            Lit::Big(_) => Kind::Big,
        }
    }

    fn cmp(&self, other: &Value) -> Ordering {
        match (&self.of, &other.of) {
            (Lit::Str(a), Lit::Str(b)) => a.cmp(b),
            (Lit::Num(a), Lit::Num(b)) => a.partial_cmp(b).expect("no NaN is a constant here"),
            (Lit::Big(a), Lit::Big(b)) => a.cmp(b),
            _ => unreachable!("a set holds values of one kind"),
        }
    }
}

/// Values of one kind from `lo` to `hi`, each `None` where there's no
/// bound, and whether it's in.
#[derive(Clone)]
struct Interval {
    lo: Option<(Value, bool)>,
    hi: Option<(Value, bool)>,
}

impl Interval {
    fn is_empty(&self) -> bool {
        match (&self.lo, &self.hi) {
            (Some((lo, a)), Some((hi, b))) => match lo.cmp(hi) {
                Ordering::Greater => true,
                Ordering::Equal => !(*a && *b),
                Ordering::Less => false,
            },
            _ => false,
        }
    }

    fn is_point(&self) -> bool {
        matches!((&self.lo, &self.hi), (Some((lo, true)), Some((hi, true))) if lo.cmp(hi) == Ordering::Equal)
    }

    fn intersect(&self, other: &Interval) -> Interval {
        // The higher low bound, the one that leaves its value out of two at one value.
        let lo = match (&self.lo, &other.lo) {
            (None, b) => b.clone(),
            (a, None) => a.clone(),
            (Some(a), Some(b)) => Some(match a.0.cmp(&b.0) {
                Ordering::Greater => a.clone(),
                Ordering::Less => b.clone(),
                Ordering::Equal => (a.0.clone(), a.1 && b.1),
            }),
        };
        let hi = match (&self.hi, &other.hi) {
            (None, b) => b.clone(),
            (a, None) => a.clone(),
            (Some(a), Some(b)) => Some(match a.0.cmp(&b.0) {
                Ordering::Less => a.clone(),
                Ordering::Greater => b.clone(),
                Ordering::Equal => (a.0.clone(), a.1 && b.1),
            }),
        };
        Interval { lo, hi }
    }

    /// `subject` within it.
    fn render(&self, subject: &Expr) -> Expr {
        if self.is_point() {
            return test(subject, Op::Eq, &self.lo.as_ref().expect("a point").0);
        }
        let lo = (self.lo.as_ref()).map(|(v, i)| test(subject, if *i { Op::Ge } else { Op::Gt }, v));
        let hi = (self.hi.as_ref()).map(|(v, i)| test(subject, if *i { Op::Le } else { Op::Lt }, v));
        all(lo.into_iter().chain(hi).collect())
    }

    /// `subject` outside it, of a kind that has no `NaN`.
    fn render_outside(&self, subject: &Expr) -> Expr {
        if self.is_point() {
            return test(subject, Op::Ne, &self.lo.as_ref().expect("a point").0);
        }
        let lo = (self.lo.as_ref()).map(|(v, i)| test(subject, if *i { Op::Lt } else { Op::Le }, v));
        let hi = (self.hi.as_ref()).map(|(v, i)| test(subject, if *i { Op::Gt } else { Op::Ge }, v));
        any(lo.into_iter().chain(hi).collect())
    }
}

/// The values of one kind a test passes: intervals apart and in order, and
/// for numbers whether `NaN` passes.
#[derive(Clone)]
struct Set {
    kind: Kind,
    pieces: Vec<Interval>,
    nan: bool,
}

impl PartialEq for Set {
    fn eq(&self, other: &Set) -> bool {
        let bound = |a: &Option<(Value, bool)>, b: &Option<(Value, bool)>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.1 == b.1 && a.0.cmp(&b.0) == Ordering::Equal,
            _ => false,
        };
        self.kind == other.kind
            && self.nan == other.nan
            && self.pieces.len() == other.pieces.len()
            && (self.pieces.iter().zip(&other.pieces)).all(|(a, b)| bound(&a.lo, &b.lo) && bound(&a.hi, &b.hi))
    }
}

impl Set {
    fn all(kind: Kind) -> Set {
        Set {
            kind,
            pieces: vec![Interval { lo: None, hi: None }],
            nan: kind == Kind::Num,
        }
    }

    fn is_empty(&self) -> bool {
        self.pieces.is_empty() && !self.nan
    }

    fn intersect(&self, other: &Set) -> Set {
        let mut pieces: Vec<Interval> = (self.pieces.iter())
            .flat_map(|a| other.pieces.iter().map(move |b| a.intersect(b)))
            .filter(|p| !p.is_empty())
            .collect();
        pieces.sort_by(|a, b| match (&a.lo, &b.lo) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Less,
            (_, None) => Ordering::Greater,
            (Some(a), Some(b)) => a.0.cmp(&b.0).then(b.1.cmp(&a.1)),
        });
        Set {
            kind: self.kind,
            pieces,
            nan: self.nan && other.nan,
        }
        .merged()
    }

    /// The values it leaves out.
    fn complement(&self) -> Set {
        let mut pieces = Vec::new();
        // Where the gap after the last piece starts, `None` of none before.
        let mut from: Option<(Value, bool)> = None;
        let mut to_the_end = true;
        for piece in &self.pieces {
            if let Some((v, i)) = &piece.lo {
                let gap = Interval {
                    lo: from.clone(),
                    hi: Some((v.clone(), !i)),
                };
                if !gap.is_empty() {
                    pieces.push(gap);
                }
            }
            match &piece.hi {
                Some((v, i)) => from = Some((v.clone(), !i)),
                None => {
                    to_the_end = false;
                    break;
                }
            }
        }
        if to_the_end {
            pieces.push(Interval { lo: from, hi: None });
        }
        Set {
            kind: self.kind,
            pieces,
            nan: self.kind == Kind::Num && !self.nan,
        }
    }

    fn union(&self, other: &Set) -> Set {
        self.complement().intersect(&other.complement()).complement()
    }

    /// Pieces that overlap or meet, one.
    fn merged(mut self) -> Set {
        let mut pieces: Vec<Interval> = Vec::new();
        for piece in self.pieces {
            if let Some(last) = pieces.last_mut()
                && meets(&last.hi, &piece.lo)
            {
                let further = match (&last.hi, &piece.hi) {
                    (None, _) => false,
                    (_, None) => true,
                    (Some(a), Some(b)) => match b.0.cmp(&a.0) {
                        Ordering::Greater => true,
                        Ordering::Equal => b.1 && !a.1,
                        Ordering::Less => false,
                    },
                };
                if further {
                    last.hi = piece.hi;
                }
                continue;
            }
            pieces.push(piece);
        }
        self.pieces = pieces;
        self
    }
}

/// Whether a piece that ends at `hi` and one that starts at `lo` leave no
/// value between them.
fn meets(hi: &Option<(Value, bool)>, lo: &Option<(Value, bool)>) -> bool {
    match (hi, lo) {
        (None, _) | (_, None) => true,
        (Some((h, hi_in)), Some((l, lo_in))) => match l.cmp(h) {
            Ordering::Less => true,
            Ordering::Equal => *hi_in || *lo_in,
            Ordering::Greater => false,
        },
    }
}

/// `bb7: { .. x = true; break bb7; .. x = false; }`, a block MIR made of a
/// value, its last statement a constant it gives where nothing else does,
/// each other written just before a `break` of it: `x = false;` first, as
/// a person gives a default, the block then left where it ends. Only
/// where nothing tells it was given first (ADR 0364, amended): each other
/// way out of the block gives its own; and what the block does before
/// its end can't see it, the variable declared in the block's list and
/// named by nothing since, or neither runs code nor throws nor leaves
/// another way.
pub(super) fn default_values(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::Labeled(label, body) = &stmts[i].kind
            && crate::lower::mir::is_block_label(label)
            && let Some((last, rest)) = body.split_last()
            && let StmtKind::Assign(target, value) = &last.kind
            && let ExprKind::Var(x) = &target.kind
            && value.is_constant()
            && let Some(given) = (Given {
                x,
                label,
                local: declared_unread(&stmts[..i], x),
            })
            .before_breaks(rest, &Inside::default())
            && js::mentions_in(rest, x) == given
        {
            let StmtKind::Labeled(_, body) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            let last = body.pop().expect("split");
            stmts.insert(i, last);
            i += 1;
        }
        i += 1;
    }
}

/// Whether `x` is declared in `before`, `let x = ..;`, and named by nothing
/// since: what can see it is only what follows in the list, which a
/// way out of it never reaches.
fn declared_unread(before: &[Stmt], x: &str) -> bool {
    (before.iter())
        .rposition(|s| matches!(&s.kind, StmtKind::Let(name, _) if name == x))
        .is_some_and(|at| js::mentions_in(&before[at + 1..], x) == 0)
}

/// A value `x` a block labeled `label` gives last, as `default_values`
/// asks of what comes before it.
struct Given<'a> {
    x: &'a str,
    label: &'a str,
    /// Whether `x` is declared in the block's list, named by nothing
    /// since (`declared_unread`): nothing but the block sees it before it
    /// ends, nor once it's left another way.
    local: bool,
}

/// Where in the block a statement is: in which of its own labeled
/// statements, and in a loop or a `switch` of its own, which a `break` of
/// no label leaves, or a loop, which a `continue` does.
#[derive(Clone, Default)]
struct Inside<'s> {
    labels: Vec<&'s str>,
    in_loop: bool,
    in_switch: bool,
}

impl<'s> Inside<'s> {
    fn labeled(&self, label: Option<&'s str>, is_loop: bool, is_switch: bool) -> Inside<'s> {
        let mut inside = self.clone();
        inside.labels.extend(label);
        inside.in_loop |= is_loop;
        inside.in_switch |= is_switch;
        inside
    }

    /// Whether a jump to `label`, or of no label, stays in the block.
    fn holds(&self, label: Option<&str>, is_continue: bool) -> bool {
        match label {
            Some(l) => self.labels.contains(&l),
            None if is_continue => self.in_loop,
            None => self.in_loop || self.in_switch,
        }
    }
}

impl Given<'_> {
    /// How many times `stmts` assign `x` just before a `break` of the
    /// block, what it's given read nowhere else: `None` where another way
    /// out of the block gives none (a `break` of it after anything else,
    /// and, of a variable that isn't local, a `return`, a `throw` or a
    /// jump past it), or, of one that isn't, where what runs before may
    /// see it given: code of the program's that it calls, or what throws.
    fn before_breaks<'s>(&self, stmts: &'s [Stmt], inside: &Inside<'s>) -> Option<usize> {
        let quiet = |e: &Expr| self.local || !e.may_throw();
        let gives = |stmt: &Stmt| matches!(&stmt.kind, StmtKind::Assign(target, _) if matches!(&target.kind, ExprKind::Var(v) if v == self.x));
        let mut given = 0;
        for (k, stmt) in stmts.iter().enumerate() {
            given += match &stmt.kind {
                StmtKind::Assign(_, value) if gives(stmt) => {
                    let breaks =
                        matches!(stmts.get(k + 1).map(|s| &s.kind), Some(StmtKind::Break(Some(l))) if l == self.label);
                    if !breaks || js::mentions_in(&[StmtKind::Expr(value.clone()).at(stmt.span)], self.x) > 0 {
                        return None;
                    }
                    if !quiet(value) {
                        return None;
                    }
                    1
                }
                // A way out of the block that gives no value of its own.
                StmtKind::Break(Some(l)) if l == self.label => {
                    if !k.checked_sub(1).is_some_and(|p| gives(&stmts[p])) {
                        return None;
                    }
                    0
                }
                StmtKind::Break(label) | StmtKind::Continue(label) => {
                    let is_continue = matches!(stmt.kind, StmtKind::Continue(_));
                    if !self.local && !inside.holds(label.as_deref(), is_continue) {
                        return None;
                    }
                    0
                }
                StmtKind::Return(_) | StmtKind::Throw(_) if !self.local => return None,
                StmtKind::If(test, a, b) => {
                    if !quiet(test) {
                        return None;
                    }
                    self.before_breaks(a, inside)? + b.as_deref().map_or(Some(0), |b| self.before_breaks(b, inside))?
                }
                StmtKind::Labeled(label, body) => {
                    self.before_breaks(body, &inside.labeled(Some(label), false, false))?
                }
                StmtKind::While { label, cond, body } => {
                    if !quiet(cond) {
                        return None;
                    }
                    self.before_breaks(body, &inside.labeled(label.as_deref(), true, false))?
                }
                StmtKind::For {
                    label,
                    start,
                    test,
                    body,
                    ..
                } => {
                    if !quiet(start) || !quiet(test) {
                        return None;
                    }
                    self.before_breaks(body, &inside.labeled(label.as_deref(), true, false))?
                }
                // Its iterator's `next` may be the program's.
                StmtKind::ForOf { label, body, .. } if self.local => {
                    self.before_breaks(body, &inside.labeled(label.as_deref(), true, false))?
                }
                StmtKind::Switch {
                    discriminant,
                    cases,
                    default,
                } => {
                    if !quiet(discriminant) || !cases.iter().flat_map(|(values, _)| values).all(&quiet) {
                        return None;
                    }
                    let inside = inside.labeled(None, false, true);
                    let mut given = 0;
                    for body in js::case_lists(cases, default) {
                        given += self.before_breaks(body, &inside)?;
                    }
                    given
                }
                // Of a variable from before, a `catch` or a `finally` may read it.
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) if self.local => {
                    self.before_breaks(a, inside)? + self.before_breaks(b, inside)?
                }
                StmtKind::Function(_) | StmtKind::Directive(_) | StmtKind::Comments => 0,
                _ if self.local || stmt.only_makes_values() => 0,
                _ => return None,
            };
        }
        Some(given)
    }
}
