//! Tidy's `if` of `&&`, `||` and `!` mixed, of the `if`s and `break`s a
//! block MIR made of its condition (`skipped_tests`, ADR 0946): what a way
//! through the condition does in its sequence (ADR 1074), and the `let`s a
//! condition sets, declared before it (A1, B1, the user's decisions).

use super::*;

/// `l: { if (p) { if (c) { if (d) { break l; } } t } } r`, MIR's of an
/// `if` whose condition the program wrote with `&&`, `||` and `!` mixed,
/// `if (p && !(c && d)) { t } r`, each `break l` a way its condition is
/// false: the one test, each part tested where it was, in the order it
/// was, and none twice (ADR 0946). Of the `if`s before `t` only, of tests
/// of a logical the program wrote; and where a path's value isn't one of
/// `&&` or `||` alone, `c ? x : y`, as it was. With `lets`, of a block
/// still there once the rest of tidy is done, a `const` its condition
/// can't read in place is a `let` set there (`falls`, A1, the user's
/// decision), never of one a later pass makes another form of.
pub(in crate::lower::mir::tidy) fn skipped_tests(stmts: &mut Vec<Stmt>, logicals: &[js::Span], lets: bool) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Labeled(label, body) = &mut stmts[i].kind else {
            i += 1;
            continue;
        };
        // The blocks it holds first, so one of them is a test of its tree:
        // `bb10: { bb7: { .. } seen += 10; }`'s `if (..) { break bb10; }`.
        skipped_tests(body, logicals, lets);
        let label = label.clone();
        let Some(test) = skipped_test(&label, &stmts[i], lets, logicals) else {
            i += 1;
            continue;
        };
        let n = test.len();
        stmts.splice(i..=i, test);
        i += n;
    }
}

/// Of a block MIR made, `label`, the one `if` `skipped_tests` makes of it.
/// Read too where a loop's body ends in it, before `continued` makes each
/// `break label` a `continue`: `if (a && (!b || c)) { for (..) { .. } }`,
/// as written, not `if (a) { if (b && !c) { continue; } for .. }`, the
/// inner loop's way out, `break label`, its end now (rd72b). With `lets`,
/// the `let`s its condition sets, which `falls` makes (A1, the user's
/// decision), declared before it: a body's names are its own till tidy is
/// done (ADR 0352), so a `let` declared a block out shadows nothing.
pub(in crate::lower::mir::tidy) fn skipped_test(
    label: &str,
    block: &Stmt,
    lets: bool,
    logicals: &[js::Span],
) -> Option<Vec<Stmt>> {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    let StmtKind::Labeled(_, body) = &block.kind else {
        return None;
    };
    if !crate::lower::mir::is_block_label(label) {
        return None;
    }
    let mut hoisting = Hoisting {
        on: lets,
        names: Vec::new(),
    };
    let (test, then, span) = skipped(body, label, &within, &mut hoisting)?;
    let mut stmts: Vec<Stmt> = (hoisting.names.into_iter())
        .map(|name| StmtKind::Let(name, None).at(js::Span::NONE))
        .collect();
    stmts.push(StmtKind::If(test, then, None).at(span));
    Some(stmts)
}

/// The `let`s `falls` and `chains` set in a condition, `(d = flags.value,
/// d * d)`, of a block's `const` it can't read in its place: whether they
/// may (`on`, of the late pass alone), and those they do, each declared
/// before the `if` (A1, B1, the user's decisions).
pub(super) struct Hoisting {
    pub(super) on: bool,
    pub(super) names: Vec<String>,
}

impl Hoisting {
    /// `name` a `let` declared before the `if`, where they may be.
    pub(super) fn declared(&mut self, name: &str) -> Option<()> {
        if !self.on {
            return None;
        }
        if !self.names.iter().any(|n| n == name) {
            self.names.push(name.to_string());
        }
        Some(())
    }
}

/// Of a block `label`'s `body`, `if (p) { tree; t }` or `tree; t`, each
/// `break label` in `tree`: the test under which `t` runs, `t`, and where
/// the first `if` is. A let chain's binding between its tests, `const x =
/// y;` of a variable or a path of properties from one, is `t`'s first,
/// each test of it a test of what it's read of: read once each test
/// before it holds, as it was, of what no test sets, the tests calling
/// nothing (ADR 0946, amended).
fn skipped(
    body: &[Stmt],
    label: &str,
    within: &dyn Fn(js::Span) -> bool,
    hoisting: &mut Hoisting,
) -> Option<(Expr, Vec<Stmt>, js::Span)> {
    let guard = |s: &Stmt| matches!(s.kind, StmtKind::If(..)) && js::names_jump(std::slice::from_ref(s), label);
    let (mut tree, mut bound, mut k) = (Vec::new(), Vec::new(), 0);
    while let Some(stmt) = body.get(k) {
        match &stmt.kind {
            _ if guard(stmt) => tree.push(stmt.clone()),
            StmtKind::Const(_, value) if rereadable(value) && body.get(k + 1).is_some_and(guard) => {
                bound.push(stmt.clone())
            }
            // A `let` a block it held declared before its test (`falls`),
            // `bb12: { let d; if (.. (d = bump(flags)) ..) { break bb12; }
            // n += d; }`: declared before this one's.
            StmtKind::Let(name, None) if tree.is_empty() && bound.is_empty() && hoisting.on => {
                hoisting.declared(name);
            }
            _ => break,
        }
        k += 1;
    }
    // Read where it was, of what no test changes: a test that runs no code.
    let mut calls = false;
    js::each_expr(&tree, &mut |e| calls |= runs_code(e));
    let read_by_tests = !bound.is_empty() && calls;
    // A test of the binding, `if (root == null)`, tests what it's read of,
    // which reads the same there: `if (inProgressRoot == null)`.
    for b in &bound {
        if let StmtKind::Const(name, value) = &b.kind {
            js::each_expr_mut(&mut tree, &mut |e| {
                if matches!(&e.kind, ExprKind::Var(v) if v == name) {
                    *e = Expr {
                        span: e.span,
                        ..value.clone()
                    };
                }
            });
        }
    }
    let then: Vec<Stmt> = bound.into_iter().chain(body[k..].iter().cloned()).collect();
    if !tree.is_empty()
        && !read_by_tests
        && body.len() > k
        && !js::names_jump(&then, label)
        && tests_within(&tree, within)
        && let Some(Truth::Test(test)) = falls(&tree, label, hoisting)
    {
        return Some((test, then, tree[0].span));
    }
    let [
        Stmt {
            kind: StmtKind::If(first, then, None),
            span,
        },
    ] = body
    else {
        return None;
    };
    let (rest, then, _) = skipped(then, label, within, hoisting)?;
    Some((Expr::bin(Op::And, first.clone(), rest), then, *span))
}

/// Whether each `if` of `tree` is a test of a logical the program wrote,
/// and what's done on a way, a call or a set, is written in one too, a
/// block's in the condition, not the `if`'s own statements (#523).
fn tests_within(tree: &[Stmt], within: &dyn Fn(js::Span) -> bool) -> bool {
    tree.iter().all(|s| match &s.kind {
        StmtKind::If(_, a, b) => {
            within(s.span) && tests_within(a, within) && b.as_deref().is_none_or(|b| tests_within(b, within))
        }
        StmtKind::Expr(_) | StmtKind::Assign(..) => within(s.span),
        _ => true,
    })
}

/// What a path through a tree of `if`s and `break`s gives: always, never,
/// or where a test holds.
enum Truth {
    Always,
    Never,
    Test(Expr),
}

/// Where `stmts`, `if`s whose branches are `if`s and `break label`s only,
/// go on past their end: each `break label` a way they don't. What a way
/// does on it, a call, a field's set or a variable's, `{ flags.set(f &
/// !32); false }` of a condition's `||`, `{ seen = 5; false }`, is done
/// where it is, in the test's sequence, which `js::Writes` reads a
/// variable's set in as it does a statement's (ADR 1088): `(s,
/// false)` where the way then leaves, `(s, true)` where it goes on, `(s,
/// t)` before a test `t` (#523). `None` of another statement, or of an
/// `if` whose branches give a test each, `c ? x : y`, which `&&` and `||`
/// don't write. A block's `let` on a way, `{ let deps =
/// this.deps.borrow().clone(); deps.unwrap() }` of a call's argument, is
/// read where it's read, once, in the test right after it, before
/// anything there does anything (`read_again`), so it runs where it ran
/// against all that does: `checkDirty($unwrap(this$.deps), this$)`, as JS
/// has no statement in a condition (#523). From the last, so one read in
/// what the next one gives is read in its place: `const value =
/// flags.value; const d = bump(flags); more(value, d + 1)` is `more(
/// flags.value, bump(flags) + 1)`, its arguments read in their order.
/// Read again, or past what does something, it's set where it was, a
/// `let` declared before the `if` (`hoisting`): `more(f * 4, { let d =
/// flags.get(); d * d })` is `((d = flags.value), more(..d * d..))`, as
/// React DOM's getModernOffsetsFromPoints writes `let next = null;` and
/// `if ((next = node.firstChild) === null)` (ReactDOMSelection.js:76, 96;
/// A1, the user's decision, 2026-11-04). Each runs where it ran: the ones
/// before it are read in their places or set before it.
fn falls(stmts: &[Stmt], label: &str, hoisting: &mut Hoisting) -> Option<Truth> {
    if stmts.iter().any(|s| matches!(s.kind, StmtKind::Const(..))) {
        let mut read = stmts.to_vec();
        let mut before = read.len();
        while let Some(k) = read[..before]
            .iter()
            .rposition(|s| matches!(s.kind, StmtKind::Const(..)))
        {
            before = k;
            if read_again(&mut read, k) {
                continue;
            }
            let StmtKind::Const(name, _) = &read[k].kind else {
                unreachable!("matched")
            };
            let name = name.clone();
            hoisting.declared(&name)?;
            let StmtKind::Const(_, value) = &read[k].kind else {
                unreachable!("matched")
            };
            let set = Expr::assigned(Expr::var(&name), value.clone());
            read[k] = StmtKind::Expr(set).at(read[k].span);
        }
        return falls(&read, label, hoisting);
    }
    let mut all = Truth::Always;
    let mut done = Vec::new();
    // What's done, then `truth`: a sequence of them where anything is.
    let after = |done: &mut Vec<Expr>, truth: Truth| match (done.is_empty(), truth) {
        (true, truth) => truth,
        (false, truth) => {
            let last = match truth {
                Truth::Always => Expr::bool(true),
                Truth::Never => Expr::bool(false),
                Truth::Test(test) => test,
            };
            Truth::Test(Expr::sequence(std::mem::take(done).into_iter().chain([last]).collect()))
        }
    };
    for stmt in stmts {
        let one = match &stmt.kind {
            StmtKind::Expr(e) => {
                done.push(e.clone());
                continue;
            }
            // A field's, or a variable's, whose set in an expression
            // `js::Writes` sees as a statement's (ADR 1088).
            StmtKind::Assign(target, value)
                if matches!(
                    target.kind,
                    ExprKind::Member(..) | ExprKind::Index(..) | ExprKind::Var(_)
                ) =>
            {
                done.push(Expr::assigned(target.clone(), value.clone()));
                continue;
            }
            StmtKind::Break(Some(l)) if l == label => after(&mut done, Truth::Never),
            StmtKind::If(test, then, otherwise) => {
                let yes = falls(then, label, hoisting)?;
                let no = falls(otherwise.as_deref().unwrap_or_default(), label, hoisting)?;
                let test = test.clone();
                let truth = match (yes, no) {
                    (Truth::Always, Truth::Always) => return None,
                    (Truth::Never, Truth::Never) => return None,
                    (Truth::Always, Truth::Never) => Truth::Test(test),
                    (Truth::Never, Truth::Always) => Truth::Test(test.negated_test()),
                    (Truth::Test(x), Truth::Always) => Truth::Test(Expr::bin(Op::Or, test.negated_test(), x)),
                    (Truth::Test(x), Truth::Never) => Truth::Test(Expr::bin(Op::And, test, x)),
                    // An `else` that tests, which tidy has made the `if`
                    // after a `then` that leaves already; and `c ? x : y`.
                    (_, Truth::Test(_)) => return None,
                };
                after(&mut done, truth)
            }
            _ => return None,
        };
        all = match (all, one) {
            (Truth::Never, _) => return None,
            (Truth::Always, one) => one,
            (Truth::Test(a), Truth::Test(b)) => Truth::Test(Expr::bin(Op::And, a, b)),
            (Truth::Test(_), Truth::Never) => return None,
            (Truth::Test(a), Truth::Always) => Truth::Test(a),
        };
    }
    Some(match (all, after(&mut done, Truth::Always)) {
        (all, Truth::Always) => all,
        (Truth::Always, done) => done,
        (Truth::Test(a), Truth::Test(b)) => Truth::Test(Expr::bin(Op::And, a, b)),
        (Truth::Never, _) | (_, Truth::Never) => return None,
    })
}
