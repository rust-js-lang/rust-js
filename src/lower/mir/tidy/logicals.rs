//! Tidy's logicals: a value the program wrote with `&&` and `||`, which MIR
//! made branches of, returned as one value as it was written, and the
//! tests of one condition joined with `&&`, a labeled block's chain of them
//! too, and where its `if` is.

use super::*;

/// `if (t) { return a; } return b;`, MIR's of a value the program wrote
/// with `&&` or `||` where `t` is tested: one value returned, `t ? a : b`,
/// as the program wrote it, each test before it folded in, `s !== "en" &&
/// (t ? a : b)`; not where it wrote a `return`. An `if`'s condition is one
/// too, as `returned_values` makes of one with an `else`.
pub(super) fn logical_returns(
    stmts: &mut Vec<Stmt>,
    logicals: &[js::Span],
    returns: &[js::Span],
    booleans: &HashSet<String>,
) {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    // The `then`'s `if`s of no `else`, each the one before's only
    // statement, of the same logical, ending in `return true;`, `if (a) {
    // if (b) { return true; } }` of `(a && b) || c`: their tests joined, `a
    // && b` (rd95). Not of another value, which `returned_values` gives its
    // written order.
    fn joined<'s>(test: &Expr, mut then: &'s [Stmt], within: &dyn Fn(js::Span) -> bool) -> (Expr, &'s [Stmt]) {
        let mut test = test.clone();
        let ends_true = |mut list: &[Stmt]| loop {
            match list {
                [
                    Stmt {
                        kind: StmtKind::If(_, inner, None),
                        ..
                    },
                ] => list = inner,
                [
                    Stmt {
                        kind: StmtKind::Return(Some(v)),
                        ..
                    },
                ] => break matches!(v.kind, ExprKind::Bool(true)),
                _ => break false,
            }
        };
        while let [
            inner @ Stmt {
                kind: StmtKind::If(t, held, None),
                ..
            },
        ] = then
            && within(inner.span)
            && ends_true(held)
        {
            test = Expr::bin(Op::And, test, t.clone());
            then = held;
        }
        (test, then)
    }
    while let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(test, then, None) = &last_if.kind
        && let (
            test,
            [
                Stmt {
                    kind: StmtKind::Return(Some(yes)),
                    span: yes_span,
                },
            ],
        ) = joined(test, then, &within)
        && within(last_if.span)
        && returned_as_written(returns, last.span, last_if.span)
        && returned_as_written(returns, *yes_span, last_if.span)
    {
        let returned = StmtKind::Return(Some(conditional(test, yes.clone(), no.clone(), booleans, false)));
        let span = last_if.span;
        stmts.splice(stmts.len() - 2.., [returned.at(span)]);
    }
}

/// Whether a `return` at `at` isn't one the program wrote, or is the
/// `return` of the logical whose test is at `test`: `return a || !b;`, its
/// two `return`s of one the program wrote around the test (ADR 0947).
fn returned_as_written(returns: &[js::Span], at: js::Span, test: js::Span) -> bool {
    let around = |r: &js::Span, s: js::Span| !s.is_none() && r.lo <= s.lo && s.hi <= r.hi;
    (returns.iter()).filter(|r| around(r, at)).all(|r| around(r, test))
}

/// `if (a) { if (b) { return c; } }`, MIR's of a `return a && b && c;` the
/// program wrote, where each test that fails goes on to a `return d` after
/// the list, a branch's of an `if` before it too, MIR's copy of what the
/// function returns past it: `return a && b && c;`, as written, each test
/// folded as `conditional` folds one (ADR 0947). React's
/// `isLegacyActEnvironment`, whose `return false` is after its `if
/// (__DEV__)`.
pub(super) fn returned_logicals(
    body: &mut Vec<Stmt>,
    logicals: &[js::Span],
    returns: &[js::Span],
    booleans: &HashSet<String>,
) {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    // The tests of a chain of `if`s with no `else`, each the next's only
    // statement, the last's a `return` of a value: the tests, the value and
    // where its `return` is.
    fn chain(stmt: &Stmt, tests: &mut Vec<(Expr, js::Span)>) -> Option<(Expr, js::Span)> {
        let StmtKind::If(test, then, None) = &stmt.kind else {
            return None;
        };
        tests.push((test.clone(), stmt.span));
        match then.as_slice() {
            [
                Stmt {
                    kind: StmtKind::Return(Some(value)),
                    span,
                },
            ] => Some((value.clone(), *span)),
            [inner] => chain(inner, tests),
            _ => None,
        }
    }
    type Fold<'f> = dyn Fn(&Stmt, &Expr) -> Option<Expr> + 'f;
    fn walk(stmts: &mut Vec<Stmt>, after: Option<&Expr>, fold: &Fold<'_>) {
        // A chain followed by the `return` its failing tests go on to, in
        // its list, MIR's copy of the one the program wrote around them:
        // one `return`, as `logical_returns` makes of one `if` (ADR 0947,
        // amended).
        let mut i = 0;
        while i + 1 < stmts.len() {
            if let StmtKind::Return(Some(no)) = &stmts[i + 1].kind
                && let Some(value) = fold(&stmts[i], no)
            {
                let span = stmts[i].span;
                stmts.splice(i..i + 2, [StmtKind::Return(Some(value)).at(span)]);
            }
            i += 1;
        }
        let n = stmts.len();
        for i in 0..n {
            let next = match stmts.get(i + 1).map(|s| &s.kind) {
                Some(StmtKind::Return(Some(value))) => Some(value.clone()),
                Some(_) => None,
                None => after.cloned(),
            };
            if i + 1 == n
                && let Some(no) = after
                && let Some(value) = fold(&stmts[i], no)
            {
                let span = stmts[i].span;
                stmts[i] = StmtKind::Return(Some(value)).at(span);
                continue;
            }
            match &mut stmts[i].kind {
                StmtKind::If(_, a, b) => {
                    walk(a, next.as_ref(), fold);
                    if let Some(b) = b {
                        walk(b, next.as_ref(), fold);
                    }
                }
                StmtKind::Labeled(_, body) => walk(body, next.as_ref(), fold),
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                    walk(body, None, fold)
                }
                StmtKind::Switch { cases, default, .. } => {
                    js::case_lists_mut(cases, default).for_each(|case| walk(case, None, fold))
                }
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    walk(a, None, fold);
                    walk(b, None, fold);
                }
                _ => {}
            }
        }
    }
    let fold = |stmt: &Stmt, no: &Expr| {
        let mut tests = Vec::new();
        let (yes, at) = chain(stmt, &mut tests)?;
        let around = |r: &js::Span, s: js::Span| !s.is_none() && r.lo <= s.lo && s.hi <= r.hi;
        // The `return` the program wrote, around each test: its value's.
        let written = returns.iter().find(|r| around(r, at))?;
        let inside = |s: js::Span| around(written, s);
        if !tests.iter().all(|&(_, s)| within(s) && inside(s)) || inside(no.span) {
            return None;
        }
        // Each test folded into an `&&` or an `||`, as the logical is
        // written (ADR 0947, amended).
        let mut value = yes;
        for (test, _) in tests.into_iter().rev() {
            value = conditional(test, value, no.clone(), booleans, false);
        }
        Some(value)
    };
    walk(body, None, &fold);
}

/// `l: { if (a) { if (b) { break l; } return true; } } return d;`, MIR's
/// of a value the program wrote with `&&` and `||` mixed, `(a && !b) || d`:
/// the one value returned, each `return` and `break` of the block a value
/// of what follows it, folded as `conditional` folds one, and a value that
/// both sides end with taken out, `c ? (p || d) : d` as `(c && p) || d`.
/// Not where what's returned would be written twice, nor where the
/// program wrote a `return` (ADR 0613).
pub(super) fn logical_blocks(
    stmts: &mut Vec<Stmt>,
    logicals: &[js::Span],
    returns: &[js::Span],
    booleans: &HashSet<String>,
) {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    let [.., block, last] = stmts.as_slice() else { return };
    let (StmtKind::Labeled(label, body), StmtKind::Return(Some(after))) = (&block.kind, &last.kind) else {
        return;
    };
    // Its tests a logical's the program wrote, the block's own span MIR's.
    if !body.first().is_some_and(|s| within(s.span)) || written(returns, last.span) {
        return;
    }
    // The value `stmts` give, falling through to `fall`, a `break` of the
    // block to `after`.
    fn value(
        stmts: &[Stmt],
        fall: &Expr,
        (label, after): (&str, &Expr),
        returns: &[js::Span],
        booleans: &HashSet<String>,
    ) -> Option<Expr> {
        match stmts {
            [] => Some(fall.clone()),
            [
                Stmt {
                    kind: StmtKind::Return(Some(v)),
                    span,
                },
                ..,
            ] if !written(returns, *span) => Some(v.clone()),
            [
                Stmt {
                    kind: StmtKind::Break(Some(l)),
                    ..
                },
                ..,
            ] if l == label => Some(after.clone()),
            [
                Stmt {
                    kind: StmtKind::If(test, then, None),
                    ..
                },
                rest @ ..,
            ] => {
                let no = value(rest, fall, (label, after), returns, booleans)?;
                let yes = value(then, &no, (label, after), returns, booleans)?;
                Some(folded(test.clone(), yes, no, booleans))
            }
            _ => None,
        }
    }
    let Some(made) = value(body, after, (label, after), returns, booleans) else {
        return;
    };
    // What's returned after it, once, as the program wrote it.
    let mut once = 0;
    let mut seen = vec![StmtKind::Expr(made.clone()).at(js::Span::NONE)];
    js::each_expr_mut(&mut seen, &mut |e| once += usize::from(e == after));
    if once > 1 || has_conditional(&made) {
        return;
    }
    let span = block.span;
    stmts.splice(stmts.len() - 2.., [StmtKind::Return(Some(made)).at(span)]);
}

/// `c ? yes : no` of booleans, as `conditional` folds it, and `c ? (p ||
/// no) : no` as `(c && p) || no`, a value both end with taken out.
fn folded(test: Expr, yes: Expr, no: Expr, booleans: &HashSet<String>) -> Expr {
    // `p` of `p || no`, an `||` chain that ends with `no`.
    fn without(e: &Expr, tail: &Expr) -> Option<Expr> {
        match &e.kind {
            ExprKind::Binary(Op::Or, a, b) if **b == *tail => Some((**a).clone()),
            ExprKind::Binary(Op::Or, a, b) => Some(Expr::bin(Op::Or, (**a).clone(), without(b, tail)?)),
            _ => None,
        }
    }
    if let Some(p) = without(&yes, &no) {
        let test = conditional(test, Expr::bool(true), Expr::bool(false), booleans, false);
        return Expr::bin(Op::Or, Expr::bin(Op::And, test, p), no);
    }
    conditional(test, yes, no, booleans, false)
}

/// Whether `e` holds a `c ? a : b`, which a fold left: not what `&&` and
/// `||` were written as.
fn has_conditional(e: &Expr) -> bool {
    let mut found = false;
    let mut stmts = vec![StmtKind::Expr(e.clone()).at(js::Span::NONE)];
    js::each_expr_mut(&mut stmts, &mut |e| found |= matches!(e.kind, ExprKind::Cond(..)));
    found
}

/// `test`, after the `const`s `bound`, each a read that does nothing, with
/// what each is read in place, those after it first: the one answer to
/// whether a test may read a value bound before it, of `if`s joined into a
/// chain. `None` where one isn't such a read, or where one is a binding's
/// read of a property, whose getter may run (`reads_a_getter`), and `test`
/// reads it twice or `again` says what's after the `const`s reads it too,
/// by its index and name: a getter's, `request.url`, runs once, as Rust's
/// `js::get` does (ADR 0672). What reads variables, or Rust's data, a
/// field, `node.count`, or a tuple's item, `pair[0]`, reads the same again.
pub(super) fn bound_into(test: &Expr, bound: &[Stmt], again: &dyn Fn(usize, &str) -> usize) -> Option<Expr> {
    let getter = reads_a_getter;
    (bound.iter().enumerate().rev()).try_fold(test.clone(), |test, (at, stmt)| match &stmt.kind {
        StmtKind::Const(name, value) if value.reads_same() => {
            let tested = js::mentions_in(&[StmtKind::Expr(test.clone()).at(js::Span::NONE)], name);
            if getter(value) && tested > 0 && tested + again(at, name) > 1 {
                return None;
            }
            test.substitute(&|v| (v == name).then(|| value.clone()))
        }
        _ => None,
    })
}

/// Of the first `bound` of `stmts`, the `const`s a chain's tests read in
/// place (`bound_into`), each nothing after reads: gone, the tests' alone.
pub(super) fn unread_bound(stmts: &mut Vec<Stmt>, bound: usize) {
    for at in (0..bound).rev() {
        if let StmtKind::Const(name, _) = &stmts[at].kind
            && js::mentions_in(&stmts[at + 1..], name) == 0
        {
            stmts.remove(at);
        }
    }
}

/// Whether `e` is a binding's read of a property, which a getter may
/// answer, as the binding's lowering says (`js::Known::Getter`, ADR 0672).
fn reads_a_getter(e: &Expr) -> bool {
    e.facts.has(js::Known::Getter)
}

/// `if (a) { if (b) { s } }`, the tests of one condition the program wrote,
/// `a && b`'s: `if (a && b) { s }`. What's bound between them, a read that
/// does nothing, comes first in `s`, and `b` reads its value, as
/// `if let`s chained with `&&` read; one only `b` read, `const captured =
/// child.type` a closure took, is gone: `b` reads it, once. Not where that
/// would read a property twice (`bound_into`): `const url = request.url;
/// if (typeof url === "string") { return url; }` stays, but where its `s`
/// breaks out to a label (`read_once_first`); nor where it would read a
/// place twice that `a` doesn't test (`read_twice_apart`).
pub(super) fn and_tests(stmts: &mut Vec<Stmt>, conditions: &[(js::Span, js::Span)]) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    let mut first_reads = Vec::new();
    for (i, stmt) in stmts.iter_mut().enumerate() {
        let span = stmt.span;
        let StmtKind::If(test, then, None) = &mut stmt.kind else {
            continue;
        };
        let [bound @ .., inner] = then.as_slice() else { continue };
        let StmtKind::If(second, body, None) = &inner.kind else {
            continue;
        };
        let one = (conditions.iter()).any(|(first, then)| within(first, span) && within(then, inner.span));
        if !one {
            continue;
        }
        let again = |at: usize, name: &str| js::mentions_in(body, name) + js::mentions_in(&bound[at + 1..], name);
        let Some(second_joined) = bound_into(second, bound, &again) else {
            first_reads.extend(read_once_first(stmt).map(|read| (i, read)));
            continue;
        };
        if read_twice_apart(test, bound, second, &again) || read_past_code(bound, second, &again) {
            continue;
        }
        // The bindings the second test reads, in their place now: not one
        // a pattern takes apart, `const a = value[0];`, the program's name
        // of what no test read.
        let tested: HashSet<String> = (bound.iter())
            .filter_map(|s| match &s.kind {
                StmtKind::Const(name, _) if second.mentions_var(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        let second = second_joined;
        let bound = bound.len();
        let Some(Stmt {
            kind: StmtKind::If(_, body, None),
            ..
        }) = then.pop()
        else {
            unreachable!("matched")
        };
        let first = std::mem::replace(test, Expr::undefined());
        *test = unimplied(Expr::bin(Op::And, first, second));
        let made = then.len() + body.len();
        then.extend(body);
        unread_bound(then, bound);
        // Each binding left that the test read, read in its place again
        // where the body reads it first (`moves::read_again`): the test's
        // code, a binding's read past which stays apart (`read_past_code`),
        // ran before it.
        let kept = bound - (made - then.len());
        for at in (0..kept).rev() {
            if matches!(&then[at].kind, StmtKind::Const(name, _) if tested.contains(name)) {
                read_again(then, at);
            }
        }
    }
    // Its name is its own in the function, as each MIR local's is, so in
    // this list too (`contracts::DECLARED_ONCE`).
    for (i, read) in first_reads.into_iter().rev() {
        stmts.insert(i, read);
    }
}

/// Whether joining `if (a) { const x = e; if (b) { s } }` into `if (a &&
/// b) { const x = e; s }` would read the place `e` twice where the chain
/// reads it once: `b` tests `x`, so the join reads `e` there, and `b` or
/// what's after reads `x` again; and `e` reads nothing `a` tests, `a` a
/// test of its own, `debug` of `if debug && let Some(value) =
/// info.value.get()`. That stays as a person writes it, `if (debug) { const
/// value = info.value; if (value != null) { .. } }` (ADR 0842). A chain that takes
/// apart what its first test tested, `const a = value[0]` of `if (value)`,
/// is joined, `if (value && value[0] < 10)`, the test and the binding of
/// one value.
fn read_twice_apart(a: &Expr, bound: &[Stmt], b: &Expr, again: &dyn Fn(usize, &str) -> usize) -> bool {
    let mut tested = HashSet::new();
    a.visit_vars(&mut |v| {
        tested.insert(v.to_string());
    });
    let test = [StmtKind::Expr(b.clone()).at(js::Span::NONE)];
    (bound.iter().enumerate()).any(|(at, stmt)| {
        let StmtKind::Const(name, value) = &stmt.kind else {
            return false;
        };
        let mut apart = true;
        value.visit_vars(&mut |v| apart &= !tested.contains(v));
        let tested = js::mentions_in(&test, name);
        apart && tested > 0 && tested + again(at, name) > 1
    })
}

/// Whether joining `if (a) { const x = e; if (b) { s } }` into `if (a &&
/// b) { const x = e; s }` would read `e` after `b` runs code, which may
/// change it: `let Some(task) = fiber.task.get() && probe(fiber)`, whose
/// `task` is what was read before `probe` cleared it. A binding `s` or a
/// later one reads stays before `b` (ADR 0842, amended).
fn read_past_code(bound: &[Stmt], b: &Expr, again: &dyn Fn(usize, &str) -> usize) -> bool {
    b.has_effects()
        && (bound.iter().enumerate())
            .any(|(at, stmt)| matches!(&stmt.kind, StmtKind::Const(name, _) if again(at, name) > 0))
}

/// Of `if (a) { const x = e; if (b) { s; break l; } }`, a chain whose `e`
/// is a property a getter may be, read once (`bound_into`), and whose `s`
/// leaves for a label, where what fails either test goes on alike: `e` read
/// first where `a` holds, `const x = a ? e : undefined;`, and the `if` left
/// `if (b) { s; break l; }`, one test, so its failures are one `else`, no
/// label to reach it, as a person reads a value once to test it. Where `b`
/// tests `x` alone, does nothing, and holds only of a value, `typeof x ===
/// "string"`, so of `undefined`, where `a` doesn't hold, it fails, as the
/// chain did (ADR 0672). The `const` to put before `stmt`, now that `if`.
fn read_once_first(stmt: &mut Stmt) -> Option<Stmt> {
    let StmtKind::If(test, then, None) = &stmt.kind else {
        return None;
    };
    let [read, inner] = then.as_slice() else { return None };
    let (StmtKind::Const(name, value), StmtKind::If(second, body, None)) = (&read.kind, &inner.kind) else {
        return None;
    };
    let mut others = false;
    second.visit_vars(&mut |v| others |= v != name);
    let breaks = matches!(body.last().map(|s| &s.kind), Some(StmtKind::Break(Some(_))));
    if !reads_a_getter(value) || others || !breaks {
        return None;
    }
    let (first, kind) = read_first(test, read, inner)?;
    stmt.kind = kind;
    Some(first)
}

/// `if (a) { const x = e; if (b) { s } }`, a chain's binding before the
/// test of it, read first: `const x = a ? e : undefined;`, and the `if`
/// it's left, `if (b) { s }`, one test, whose failures are one `else`. The
/// one rule of when that's the chain, which `read_once_first` and
/// `chain_values` ask: `e` is read once, where `a` holds, as it was; and
/// `b`, which does nothing and holds only where `x` is a value, `x !=
/// null`, `typeof x === "string"` or `x` (`implies`), fails of `undefined`,
/// where `a` doesn't hold, as the chain did; and `a` reads no other `x`,
/// which the `const` would now be (ADRs 0672 and 0870).
fn read_first(test: &Expr, read: &Stmt, inner: &Stmt) -> Option<(Stmt, StmtKind)> {
    let (StmtKind::Const(name, value), StmtKind::If(second, _, None)) = (&read.kind, &inner.kind) else {
        return None;
    };
    let defined = Expr::bin(Op::LooseNe, Expr::var(name), Expr::null());
    let mut tests = Vec::new();
    and_parts(second, &mut tests);
    if second.has_effects() || !tests.iter().any(|t| implies(t, &defined)) || test.mentions_var(name) {
        return None;
    }
    let first = StmtKind::Const(name.clone(), Expr::cond(test.clone(), value.clone(), Expr::undefined())).at(read.span);
    Some((first, inner.kind.clone()))
}

/// A value a let chain of one binding gives, `if a && let Some(x) = e { t }
/// else { f }`, each branch one value, a thread-local's or a `let`'s: `x`
/// read first (`read_first`), `const x = a ? e : undefined;`, as an `if
/// let`'s is, and its `if` then of one test and two values, which
/// `branch_values` and `returned_values` make one, `x != null ? t : f`
/// (ADR 0870): not the chain's two `if`s and the `let` they assign, which a
/// thread-local's closure called where it's made returns. MIR's of it: `l:
/// { if (a) { const x = e; if (x != null) { v = t; break l; } } v = f; }`,
/// or of what's returned, `if (a) { .. { return t; } } return f;`, neither
/// a `return` the program wrote; nor where the list names `x` anywhere
/// else. A place's read MIR makes no `const` of, `if (s.width != null)`,
/// which `and_tests` makes one test of.
pub(super) fn chain_values(stmts: &mut Vec<Stmt>, returns: &[js::Span]) {
    for stmt in stmts.iter_mut() {
        let StmtKind::Labeled(label, body) = &mut stmt.kind else {
            continue;
        };
        let label = Some(label.as_str());
        let made = match body.as_slice() {
            [chain, rest @ ..] => valued_chain(chain, rest, label, returns, &body[1..]),
            [] => None,
        };
        if let Some((first, test)) = made {
            body[0] = test;
            body.insert(0, first);
        }
    }
    let Some(at) = stmts.len().checked_sub(2) else {
        return;
    };
    let (before, chain) = stmts.split_at(at);
    let others: Vec<Stmt> = before.iter().chain(&chain[1..]).cloned().collect();
    if let Some((first, test)) = valued_chain(&chain[0], &chain[1..], None, returns, &others) {
        stmts[at] = test;
        stmts.insert(at, first);
    }
}

/// Of `chain`, `if (a) { const x = e; if (b) { .. } }`, followed by `rest`
/// in a list whose other statements are `others`, and of a block `label`
/// where the list is one's: what `chain_values` makes of it, each branch
/// one value, the `const` where the chain is, as the comment before the
/// chain is before the read it says of, and the test where it was.
fn valued_chain(
    chain: &Stmt,
    rest: &[Stmt],
    label: Option<&str>,
    returns: &[js::Span],
    others: &[Stmt],
) -> Option<(Stmt, Stmt)> {
    // The tests before the binding, a level each, `a && b && let ..`'s.
    fn levels<'s>(stmt: &'s Stmt, tests: &mut Vec<Expr>) -> Option<(&'s Stmt, &'s Stmt)> {
        let StmtKind::If(test, then, None) = &stmt.kind else {
            return None;
        };
        tests.push(test.clone());
        match then.as_slice() {
            [read, inner] if matches!(read.kind, StmtKind::Const(..)) => Some((read, inner)),
            [inner] => levels(inner, tests),
            _ => None,
        }
    }
    let mut tests = Vec::new();
    let (read, inner) = levels(chain, &mut tests)?;
    let (StmtKind::Const(name, _), StmtKind::If(_, body, None)) = (&read.kind, &inner.kind) else {
        return None;
    };
    // Its value, each way: what's left once `x`'s copies, `const k = x;`,
    // the pattern's, are.
    let aliases = body
        .iter()
        .take_while(|s| matches!(&s.kind, StmtKind::Const(_, v) if matches!(&v.kind, ExprKind::Var(x) if x == name)));
    let yes = one_value(&body[aliases.count()..], label, returns, false)?;
    let no = one_value(rest, label, returns, true)?;
    let one = match (yes, no) {
        (Valued::Assigned(x, yes), Valued::Assigned(y, no)) => x == y && small(yes) && small(no),
        (Valued::Returned(yes), Valued::Returned(no)) => small(yes) && small(no) && folds_into_return(yes, no),
        _ => false,
    };
    if !one {
        return None;
    }
    if js::mentions_in(others, name) > 0 || declared(others).any(|n| n == name) {
        return None;
    }
    let test = (tests.into_iter())
        .reduce(|a, b| Expr::bin(Op::And, a, b))
        .expect("one");
    let (first, test) = read_first(&test, read, inner)?;
    Some((first.kind.at(chain.span), test.at(inner.span)))
}

/// What a branch of a chain's value gives, of `label`'s block.
enum Valued<'s> {
    Assigned(&'s str, &'s Expr),
    Returned(&'s Expr),
}

/// Of `stmts`, a branch's: one value, `v = t;` and its `break` to `label`,
/// or `v = f;` the block ends with, `last`, or `return t;` the program
/// didn't write.
fn one_value<'s>(stmts: &'s [Stmt], label: Option<&str>, returns: &[js::Span], last: bool) -> Option<Valued<'s>> {
    let assigned = |s: &'s Stmt| match &s.kind {
        StmtKind::Assign(target, value) => match &target.kind {
            ExprKind::Var(x) => Some(Valued::Assigned(x, value)),
            _ => None,
        },
        _ => None,
    };
    match stmts {
        [
            Stmt {
                kind: StmtKind::Return(Some(value)),
                span,
            },
        ] if label.is_none() && !written(returns, *span) => Some(Valued::Returned(value)),
        [assign] if label.is_some() && last => assigned(assign),
        [
            assign,
            Stmt {
                kind: StmtKind::Break(Some(l)),
                ..
            },
        ] if Some(l.as_str()) == label => assigned(assign),
        _ => None,
    }
}

/// Each test `a && b && c` is of, in order.
fn and_parts<'e>(test: &'e Expr, parts: &mut Vec<&'e Expr>) {
    match &test.kind {
        ExprKind::Binary(Op::And, a, b) => {
            and_parts(a, parts);
            and_parts(b, parts);
        }
        _ => parts.push(test),
    }
}

/// An `&&` chain without each part a later one implies: `typeof x ===
/// "number"` of `x != null && typeof x === "number"`, as `implies` tells,
/// where the part does nothing but test (ADR 0614).
fn unimplied(test: Expr) -> Expr {
    fn parts(e: Expr, out: &mut Vec<Expr>) {
        match e.kind {
            ExprKind::Binary(Op::And, a, b) => {
                parts(*a, out);
                parts(*b, out);
            }
            _ => out.push(e),
        }
    }
    let mut all = Vec::new();
    parts(test, &mut all);
    let kept: Vec<Expr> = (0..all.len())
        .filter(|&k| all[k].has_effects() || !all[k + 1..].iter().any(|by| implies(by, &all[k])))
        .map(|k| all[k].clone())
        .collect();
    (kept.into_iter())
        .reduce(|a, b| Expr::bin(Op::And, a, b))
        .expect("the last part is kept")
}

/// `if (a) { continue; } if (b) { continue; }`, MIR's of an `if` whose
/// condition the program wrote `a || b`, each part's way to the one jump
/// its branch is: `if (a || b) { continue; }`, as it's written, React
/// DOM's hydrateHoistable's `if (node.getAttribute("href") !== (..) || ..)`
/// (rd225, #413). The same `b` once `a` is false, as `||` tests it. Only
/// where each test is of the one condition of `ifs`, the smallest that
/// holds it or that it is, MIR's test of an `if` that one; and of a jump
/// alike in each, a `continue`, a `break` or a `return` of nothing.
pub(super) fn written_ors(stmts: &mut Vec<Stmt>, ifs: &[js::Span]) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    let written = |s: js::Span| {
        (ifs.iter())
            .filter(|c| within(c, s) || (within(&s, **c) && s.hi == c.hi))
            .min_by_key(|c| c.hi - c.lo)
            .copied()
    };
    let one = |a: js::Span, b: js::Span| written(a).is_some_and(|c| written(b) == Some(c));
    let jump = |stmts: &[Stmt]| match stmts {
        [stmt] => matches!(
            stmt.kind,
            StmtKind::Continue(_) | StmtKind::Break(_) | StmtKind::Return(None)
        ),
        _ => false,
    };
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let StmtKind::If(a, then, None) = &stmts[i].kind
            && let StmtKind::If(b, other, None) = &stmts[i + 1].kind
            && jump(then)
            && jump(other)
            && super::decisions::same_stmt(&then[0], &other[0])
            && one(a.span, b.span)
        {
            let b = b.clone();
            stmts.remove(i + 1);
            if let StmtKind::If(a, ..) = &mut stmts[i].kind {
                let span = a.span;
                *a = Expr::bin(Op::Or, std::mem::replace(a, Expr::undefined()), b).or_at(span);
            }
            continue;
        }
        i += 1;
    }
}

/// `l: { if (p) { if (c) { if (d) { break l; } } t } } r`, MIR's of an
/// `if` whose condition the program wrote with `&&`, `||` and `!` mixed,
/// `if (p && !(c && d)) { t } r`, each `break l` a way its condition is
/// false: the one test, each part tested where it was, in the order it
/// was, and none twice (ADR 0946). Of the `if`s before `t` only, of tests
/// of a logical the program wrote; and where a path's value isn't one of
/// `&&` or `||` alone, `c ? x : y`, as it was.
pub(super) fn skipped_tests(stmts: &mut [Stmt], logicals: &[js::Span]) {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    for stmt in stmts.iter_mut() {
        let StmtKind::Labeled(label, body) = &stmt.kind else {
            continue;
        };
        if !crate::lower::mir::is_block_label(label) {
            continue;
        }
        let Some((test, then, span)) = skipped(body, label, &within) else {
            continue;
        };
        *stmt = StmtKind::If(test, then, None).at(span);
    }
}

/// Of a block `label`'s `body`, `if (p) { tree; t }` or `tree; t`, each
/// `break label` in `tree`: the test under which `t` runs, `t`, and where
/// the first `if` is. A let chain's binding between its tests, `const x =
/// y;` of a variable or a path of properties from one, is `t`'s first,
/// each test of it a test of what it's read of: read once each test
/// before it holds, as it was, of what no test sets, the tests calling
/// nothing (ADR 0946, amended).
fn skipped(body: &[Stmt], label: &str, within: &dyn Fn(js::Span) -> bool) -> Option<(Expr, Vec<Stmt>, js::Span)> {
    let guard = |s: &Stmt| matches!(s.kind, StmtKind::If(..)) && js::names_jump(std::slice::from_ref(s), label);
    let (mut tree, mut bound, mut k) = (Vec::new(), Vec::new(), 0);
    while let Some(stmt) = body.get(k) {
        match &stmt.kind {
            _ if guard(stmt) => tree.push(stmt.clone()),
            StmtKind::Const(_, value) if read_of_a_path(value) && body.get(k + 1).is_some_and(guard) => {
                bound.push(stmt.clone())
            }
            _ => break,
        }
        k += 1;
    }
    // Read where it was, of what no test changes: a test of no call.
    let mut calls = false;
    js::each_expr(&tree, &mut |e| {
        calls |= matches!(
            e.kind,
            ExprKind::Call(..)
                | ExprKind::New(..)
                | ExprKind::Assigned(..)
                | ExprKind::Stepped(..)
                | ExprKind::Await(_)
        )
    });
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
        && let Some(Truth::Test(test)) = falls(&tree, label)
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
    let (rest, then, _) = skipped(then, label, within)?;
    Some((Expr::bin(Op::And, first.clone(), rest), then, *span))
}

/// Whether `e` reads a variable, or a path of properties from one, `a.b`.
fn read_of_a_path(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Var(_) | ExprKind::Symbol(_) => true,
        ExprKind::Member(object, _) => read_of_a_path(object),
        _ => false,
    }
}

/// Whether each `if` of `tree` is a test of a logical the program wrote.
fn tests_within(tree: &[Stmt], within: &dyn Fn(js::Span) -> bool) -> bool {
    tree.iter().all(|s| match &s.kind {
        StmtKind::If(_, a, b) => {
            within(s.span) && tests_within(a, within) && b.as_deref().is_none_or(|b| tests_within(b, within))
        }
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
/// go on past their end: each `break label` a way they don't. `None` of
/// another statement, or of an `if` whose branches give a test each, `c ?
/// x : y`, which `&&` and `||` don't write.
fn falls(stmts: &[Stmt], label: &str) -> Option<Truth> {
    let mut all = Truth::Always;
    for stmt in stmts {
        let one = match &stmt.kind {
            StmtKind::Break(Some(l)) if l == label => Truth::Never,
            StmtKind::If(test, then, otherwise) => {
                let yes = falls(then, label)?;
                let no = falls(otherwise.as_deref().unwrap_or_default(), label)?;
                let test = test.clone();
                match (yes, no) {
                    (Truth::Always, Truth::Always) => return None,
                    (Truth::Never, Truth::Never) => return None,
                    (Truth::Always, Truth::Never) => Truth::Test(test),
                    (Truth::Never, Truth::Always) => Truth::Test(test.negated_test()),
                    (Truth::Test(x), Truth::Always) => Truth::Test(Expr::bin(Op::Or, test.negated_test(), x)),
                    (Truth::Test(x), Truth::Never) => Truth::Test(Expr::bin(Op::And, test, x)),
                    // An `else` that tests, which tidy has made the `if`
                    // after a `then` that leaves already; and `c ? x : y`.
                    (_, Truth::Test(_)) => return None,
                }
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
    Some(all)
}

/// Of `l: { if (a) { if (b) { s; break l; } } t }`: its tests, `[a, b]`,
/// `s` and `t`, where `l` is named nowhere else.
pub(super) fn or_chain(body: &[Stmt], label: &str) -> Option<(Vec<Expr>, Vec<Stmt>, Vec<Stmt>)> {
    let [first, then @ ..] = body else { return None };
    let (tests, otherwise) = broken_chain(std::slice::from_ref(first), label)?;
    if tests.is_empty() || js::names_jump(then, label) || js::names_jump(&otherwise, label) {
        return None;
    }
    Some((tests, otherwise, then.to_vec()))
}

/// Of `if (a) { if (b) { s; break l; } }`: `[a, b]` and `s`.
pub(super) fn broken_chain(stmts: &[Stmt], label: &str) -> Option<(Vec<Expr>, Vec<Stmt>)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, otherwise) = broken_chain(inner, label)?;
            tests.insert(0, test.clone());
            Some((tests, otherwise))
        }
        [
            rest @ ..,
            Stmt {
                kind: StmtKind::Break(Some(l)),
                ..
            },
        ] if l == label => Some((Vec::new(), rest.to_vec())),
        _ => None,
    }
}

/// Where the `if` a labeled block's chain is made is, of `&&`'s and `||`'s
/// or of a match's tree (`decisions`): where the program wrote the block,
/// `'desc: {`, its `span`; of one MIR's structure made, of no Rust of its
/// own, where its first `if` is, the chain's first code, not at the next
/// statement's start, as a statement of none is, nor at its first arm's
/// code (ADRs 0865 and 0444, amended).
pub(super) fn made_where(span: js::Span, body: &[Stmt]) -> js::Span {
    match body.first() {
        Some(first) if span.is_none() => first.span,
        _ => span,
    }
}

/// A test the program wrote `!(a && b)`, which MIR's branches made `!a ||
/// !b`, as it wrote it: `!(a && b)`, where every test of the chain is in
/// what a `!` the program wrote negates, the innermost `&&` or `||` it
/// wrote around them, and the chain is the other of the two
/// (`Written::negations`). Of a test whose truth alone is read, an `if`'s,
/// a `while`'s or a conditional's, which De Morgan's laws make the same
/// either way (ADR 0879, amended).
pub(super) fn written_negations(body: &mut Vec<Stmt>, negations: &[(js::Span, bool)], logicals: &[js::Span]) {
    if negations.is_empty() {
        return;
    }
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::If(test, ..) | StmtKind::While { cond: test, .. } = &mut stmt.kind {
                negated_as_written(test, negations, logicals);
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Cond(test, ..) = &mut e.kind {
            negated_as_written(test, negations, logicals);
        }
    });
}

/// `written_negations` of one test, or of the chains inside it: of the
/// parts of a chain of one operator, `a && !b && !c`, each run of them
/// written as one negation, `a && !(b || c)`, which MIR's chain joined.
fn negated_as_written(test: &mut Expr, negations: &[(js::Span, bool)], logicals: &[js::Span]) {
    let ExprKind::Binary(op @ (Op::And | Op::Or), ..) = test.kind else {
        return;
    };
    let written = |parts: &[Expr]| written_negation(parts, op == Op::Or, negations, logicals);
    let joined = |parts: Vec<Expr>| {
        parts
            .into_iter()
            .reduce(|all, next| Expr::bin(op, all, next))
            .expect("parts")
    };
    let mut parts = Vec::new();
    operands(std::mem::replace(test, Expr::undefined()), op, &mut parts);
    let mut kept = Vec::new();
    let mut at = 0;
    while at < parts.len() {
        // The longest run from here the program wrote as one `!( .. )`.
        let run = (at + 2..=parts.len()).rev().find(|&end| written(&parts[at..end]));
        match run {
            Some(end) => {
                let chain = joined(parts[at..end].to_vec());
                kept.push(Expr::unary(UnaryOp::Not, chain.negated_test()));
                at = end;
            }
            None => {
                let mut part = parts[at].clone();
                negated_as_written(&mut part, negations, logicals);
                kept.push(part);
                at += 1;
            }
        }
    }
    *test = joined(kept);
}

/// Whether `parts`, joined with `&&` where `and`, else `||`, are of a
/// negation the program wrote of the other operator, `!(a || b)` of `!a
/// && !b`: each written inside the operand of a `!` the program wrote, the
/// innermost `&&` or `||` written around them (`Written::negations`).
fn written_negation(parts: &[Expr], and: bool, negations: &[(js::Span, bool)], logicals: &[js::Span]) -> bool {
    let mut spans = Vec::new();
    let within = |outer: js::Span, s: &js::Span| outer.lo <= s.lo && s.hi <= outer.hi;
    parts.iter().all(|part| tests_of(part, &mut spans))
        && (logicals.iter())
            .filter(|&&l| spans.iter().all(|s| within(l, s)))
            .min_by_key(|l| l.hi - l.lo)
            .is_some_and(|&inner| negations.contains(&(inner, and)))
}

/// `!(status === "a" || status === "b")`, a `!` of a chain the program
/// didn't write as one, `!matches!(status, Some("a" | "b"))`'s of its
/// pattern's alternatives: each part's negated test, De Morgan's, `status
/// !== "a" && status !== "b"`, as a person writes it, where each part is
/// an equality's test or a `!`'s, which negates exactly, in a value too,
/// as `!` and each part are `bool`s alike. A `!( .. )` the program wrote
/// of an `&&` or an `||` stays (`written_negation`, ADR 0879, amended).
pub(super) fn unwritten_negations(body: &mut [Stmt], negations: &[(js::Span, bool)], logicals: &[js::Span]) {
    // Whether each test of the chain `e` negates exactly.
    fn exact(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Binary(Op::And | Op::Or, a, b) => exact(a) && exact(b),
            ExprKind::Binary(Op::Eq | Op::Ne | Op::LooseEq | Op::LooseNe, ..) | ExprKind::Unary(UnaryOp::Not, _) => {
                true
            }
            _ => false,
        }
    }
    js::each_expr_mut(body, &mut |e| {
        let ExprKind::Unary(UnaryOp::Not, chain) = &e.kind else {
            return;
        };
        let ExprKind::Binary(op @ (Op::And | Op::Or), ..) = chain.kind else {
            return;
        };
        let mut parts = Vec::new();
        operands((**chain).clone(), op, &mut parts);
        if !exact(chain) || written_negation(&parts, op == Op::And, negations, logicals) {
            return;
        }
        let ExprKind::Unary(_, chain) = std::mem::replace(&mut e.kind, ExprKind::Undefined) else {
            unreachable!("a `!` of a chain");
        };
        *e = Expr {
            span: e.span,
            ..chain.negated_test()
        };
    });
}

/// The parts of a chain of `op`, `a && b && c`'s `a`, `b` and `c`, in order.
fn operands(e: Expr, op: Op, parts: &mut Vec<Expr>) {
    match e.kind {
        ExprKind::Binary(o, a, b) if o == op => {
            operands(*a, op, parts);
            operands(*b, op, parts);
        }
        kind => parts.push(Expr { kind, ..e }),
    }
}

/// Where each test of a chain of `&&` and `||` is written, a negation's
/// where its operand is: whether each is.
fn tests_of(e: &Expr, spans: &mut Vec<js::Span>) -> bool {
    match &e.kind {
        ExprKind::Binary(Op::And | Op::Or, a, b) => tests_of(a, spans) && tests_of(b, spans),
        _ if !e.span.is_none() => {
            spans.push(e.span);
            true
        }
        ExprKind::Unary(UnaryOp::Not, x) if !matches!(x.kind, ExprKind::Binary(Op::And | Op::Or, ..)) => {
            tests_of(x, spans)
        }
        _ => false,
    }
}

/// A test of a bit set's bits against `0`, `(lanes & X) !== 0`, as JS
/// writes it, `lanes & X`, and `=== 0`'s `!(lanes & X)`: a bitwise
/// operator's value is a number, an int32's or a `u32`'s, never `NaN`,
/// so it's truthy where it isn't `0`. A `u32`'s wrap of one, `(a & b) >>>
/// 0`, is `0` where what it wraps is. And `!== 0` of an integer's, a
/// variable each of whose locals is one, `numbers`, or a wrap, never `NaN`
/// either: `if (lanes)`; its `=== 0` stays, `i === 0`, as JS compares a
/// count, and so does a remainder's, `n % 2 === 0`. Of a test whose truth alone is read,
/// an `if`'s, a `while`'s or a conditional's, and the parts of an `&&`,
/// `||` or `!` of one (ADR 0896, amended).
pub(super) fn bit_tests(body: &mut Vec<Stmt>, numbers: &HashSet<String>, patterns: &[js::Span]) {
    // Not a name a closure in the body declares too, which may be a
    // float's: the walk of a conditional goes into closures.
    let mut declared = HashSet::new();
    js::each_expr(body, &mut |e| match &e.kind {
        ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
            declared.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
            js::each_declared(body, &mut |name| _ = declared.insert(name.to_string()));
        }
        ExprKind::Function(function) => {
            declared.extend(function.params.iter().flat_map(|p| p.names()).map(str::to_string));
            js::each_declared(&function.body, &mut |name| _ = declared.insert(name.to_string()));
        }
        _ => {}
    });
    let integer = |name: &str| numbers.contains(name) && !declared.contains(name);
    // Not a `match`'s pattern's, `0 | 6 =>`, which compares what's matched
    // with its constants, as a `switch`'s `case 0:` would.
    let matched = |span: js::Span| {
        !span.is_none() && (patterns.iter()).any(|pattern| pattern.lo <= span.lo && span.lo < pattern.hi)
    };
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            let span = stmt.span;
            if let StmtKind::If(test, ..) | StmtKind::While { cond: test, .. } = &mut stmt.kind {
                bit_test(test, &integer, &matched, span);
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        let span = e.span;
        if let ExprKind::Cond(test, ..) = &mut e.kind {
            bit_test(test, &integer, &matched, span);
        }
    });
}

/// Whether `e` is an integer's value, never `NaN`, whose test `!== 0` is
/// its truth: a variable each of whose locals is one, or a sum's,
/// difference's or product's wrap, an `i32`'s `(a + b) | 0` or a `u32`'s
/// `(a - b) >>> 0`, a number of 32 bits. Not a remainder's, wrapped or
/// not, `n % 2 !== 0` or `$rem(n, m) | 0`: `n % 2 === 0` is how JS tests
/// one (ADR 0896, amended).
fn integer_value(e: &Expr, integer: &dyn Fn(&str) -> bool) -> bool {
    match &e.kind {
        ExprKind::Var(name) => integer(name),
        ExprKind::Binary(Op::BitOr | Op::UShr, wrapped, zero) if zero.as_int() == Some(0) => matches!(
            wrapped.kind,
            ExprKind::Binary(Op::Add | Op::Sub | Op::Mul, ..) | ExprKind::Unary(UnaryOp::Neg, _)
        ),
        _ => false,
    }
}

/// `bit_tests` of one test, and of the tests it's made of.
/// `matched` says whether a test, where it's written, its own span or
/// `around`'s, the statement's or conditional's it's of, is a `match`'s pattern's.
fn bit_test(test: &mut Expr, integer: &dyn Fn(&str) -> bool, matched: &dyn Fn(js::Span) -> bool, around: js::Span) {
    let at = if test.span.is_none() { around } else { test.span };
    match &mut test.kind {
        ExprKind::Binary(Op::And | Op::Or, a, b) => {
            bit_test(a, integer, matched, at);
            bit_test(b, integer, matched, at);
        }
        ExprKind::Unary(UnaryOp::Not, a) => bit_test(a, integer, matched, at),
        ExprKind::Binary(op @ (Op::Ne | Op::Eq), a, b)
            if b.as_int() == Some(0) && (bits(a) || (*op == Op::Ne && !matched(at) && integer_value(a, integer))) =>
        {
            let ne = *op == Op::Ne;
            let bits = match std::mem::replace(&mut **a, Expr::undefined()) {
                Expr {
                    kind: ExprKind::Binary(Op::UShr, x, zero),
                    ..
                } if zero.as_int() == Some(0) && bits(&x) => *x,
                bits => bits,
            };
            let (span, kept) = (test.span, test.facts.of_statement());
            *test = match ne {
                true => bits,
                false => Expr::unary(UnaryOp::Not, bits),
            };
            // What the test says of its `if`, its written `else` (ADR 0237,
            // amended).
            test.facts = test.facts.and(kept);
            if !span.is_none() {
                test.span = span;
            }
        }
        _ => {}
    }
}

/// Whether `e` is a bitwise operator's value, a number of 32 bits; not an
/// `i32`'s wrap, `(n % 2) | 0`, which is the arithmetic's.
fn bits(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Binary(Op::BitOr, _, zero) if zero.as_int() == Some(0) => false,
        ExprKind::Binary(Op::UShr, x, zero) if zero.as_int() == Some(0) => bits(x),
        ExprKind::Binary(Op::BitAnd | Op::BitOr | Op::BitXor | Op::Shl | Op::Shr | Op::UShr, ..) => true,
        ExprKind::Unary(UnaryOp::BitNot, _) => true,
        _ => false,
    }
}

/// `l: { if (c) { if (a) { if (b) { s; break l; } } t } }`, `branch_values`'
/// `&&` chain (`or_chain`) in `if`s the block is all of, where the block goes
/// on after them: `if (c) { if (a && b) { s } else { t } }`, a let chain's
/// with an `else` inside an `if` (ADR 0946, amended). After the passes that
/// name its temporaries and join its `if`s, which the chain's tests then are.
pub(super) fn chains_in_blocks(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::Labeled(label, body) = &mut stmt.kind else {
            continue;
        };
        if !crate::lower::mir::is_block_label(label) || !chain_within(body, label) {
            continue;
        }
        let (label, mut body) = (label.clone(), std::mem::take(body));
        fold_within(&mut body, &label);
        *stmt = body.pop().expect("chain_within's `if`");
    }
}

/// `chains_in_blocks`' fold of `list`, `chain_within` it: the chain where
/// it is, in the innermost `if`, after the bindings before it.
fn fold_within(list: &mut [Stmt], label: &str) {
    let [
        Stmt {
            kind: StmtKind::If(_, inner, None),
            ..
        },
    ] = list
    else {
        unreachable!("chain_within")
    };
    let Some((k, chain)) = chained(inner, label) else {
        return fold_within(inner, label);
    };
    let Some((tests, otherwise, then)) = or_chain(&chain, label) else {
        unreachable!("chained")
    };
    let test = (tests.into_iter())
        .rev()
        .reduce(|b, a| Expr::bin(Op::And, a, b))
        .expect("one");
    let span = made_where(js::Span::NONE, &chain);
    inner.truncate(k);
    inner.push(
        match then.is_empty() {
            true => StmtKind::If(test, otherwise, None),
            false => StmtKind::If(test.negated_test(), then, Some(otherwise)),
        }
        .at(span),
    );
}

/// Of `list`, a block's: how many bindings its chain follows, and the
/// chain, `or_chain`'s, of a let chain's bindings between its tests, each
/// `const x = y;` of a variable, out of it (`hoisted`), where the tests call
/// nothing, so the variable reads the same before them.
fn chained(list: &[Stmt], label: &str) -> Option<(usize, Vec<Stmt>)> {
    let k = (list.iter())
        .take_while(|s| matches!(&s.kind, StmtKind::Const(_, value) if read_of_a_path(value)))
        .count();
    let mut chain = list[k..].to_vec();
    // Whether the chain's tests, or the bindings between them, call
    // anything: not what its body does once they've held.
    fn calls(stmt: &Stmt) -> bool {
        let StmtKind::If(test, then, None) = &stmt.kind else {
            return false;
        };
        let mut found = false;
        let mut called = |e: &Expr| {
            found |= matches!(
                e.kind,
                ExprKind::Call(..) | ExprKind::New(..) | ExprKind::Assigned(..) | ExprKind::Await(_)
            )
        };
        test.each(&mut called);
        let k = (then.iter())
            .take_while(|s| matches!(&s.kind, StmtKind::Const(_, value) if read_of_a_path(value)))
            .count();
        then[..k].iter().for_each(|s| {
            if let StmtKind::Const(_, value) = &s.kind {
                value.each(&mut called);
            }
        });
        found || matches!(&then[k..], [next] if calls(next))
    }
    if let Some(first) = chain.first_mut()
        && !calls(first)
    {
        sunk(first, Vec::new());
    }
    or_chain(&chain, label).map(|_| (k, chain))
}

/// A let chain's bindings at the start of each `then` of `if (a) { const
/// x = y.v; if (x != null) { s } }`, `carried` those above: in its last
/// `then`, each test of one a test of what it reads, `if (a) { if (y.v !=
/// null) { const x = y.v; s } }`, read only where each test before it
/// held, as it was, and the same, as the tests call nothing (ADR 0946,
/// amended).
fn sunk(stmt: &mut Stmt, mut carried: Vec<Stmt>) {
    let StmtKind::If(_, then, None) = &mut stmt.kind else {
        return;
    };
    let k = (then.iter())
        .take_while(|s| matches!(&s.kind, StmtKind::Const(_, value) if read_of_a_path(value)))
        .count();
    if !matches!(
        &then[k..],
        [Stmt {
            kind: StmtKind::If(..),
            ..
        }]
    ) {
        then.splice(0..0, carried);
        return;
    }
    carried.extend(then.drain(..k));
    if let StmtKind::If(test, ..) = &mut then[0].kind {
        let read = |n: &str| {
            (carried.iter()).rev().find_map(|c| match &c.kind {
                StmtKind::Const(name, value) if name == n => Some(value.clone()),
                _ => None,
            })
        };
        if let Some(tested) = test.substitute(&read) {
            *test = tested;
        }
    }
    sunk(&mut then[0], carried);
}

/// Whether `body`, a block `label`'s, is an `if` of no `else` alone, of
/// such an `if` or of `or_chain`'s `&&` chain (`chained`), to the block's
/// end: the chain each `if` is all of the block within (`chains_in_blocks`).
fn chain_within(body: &[Stmt], label: &str) -> bool {
    matches!(body, [Stmt { kind: StmtKind::If(_, inner, None), .. }]
        if chained(inner, label).is_some() || chain_within(inner, label))
}

/// Whether `yes` and `no` are the same value, read the same either way:
/// one constant, `5` and `5` (not `0` and `-0`, which JS tells apart), or
/// one path of what does nothing. An `if` of such branches whose test does
/// something stays an `if`, `if (row.length >= 1 && mark(row[0])) { level
/// = 5; } else { level = 5; }`, as the program wrote its two arms, not `c ?
/// 5 : 5`; one whose test does nothing is the value (`conditional`).
pub(super) fn alike_branches(yes: &Expr, no: &Expr) -> bool {
    match (&yes.kind, &no.kind) {
        (ExprKind::Num(a), ExprKind::Num(b)) => a.to_bits() == b.to_bits(),
        (ExprKind::Str(a), ExprKind::Str(b)) => a == b,
        (ExprKind::Bool(a), ExprKind::Bool(b)) => a == b,
        (ExprKind::BigInt(a), ExprKind::BigInt(b)) => a == b,
        (ExprKind::Null, ExprKind::Null) | (ExprKind::Undefined, ExprKind::Undefined) => true,
        _ => !yes.has_effects() && !no.has_effects() && js::same_path(yes, no),
    }
}

/// `test ? yes : no`: `test && yes` where `no` is `false`, and `test || no`
/// where `yes` is `true`, the test `!!test` where it's not a `bool`. Of an
/// `if` the program wrote with an `else`, `written_if`, `c ? b : false` is
/// `c && b` and `c ? true : b` `c || b` where `c` and `b` are `bool`s, the
/// user's decision; other branches, `c ? false : b`, and a test or a branch
/// that isn't a `bool`, an `Option<bool>`'s `c ? o : false`, as it wrote
/// them (ADR 0879, amended).
pub(super) fn conditional(test: Expr, yes: Expr, no: Expr, booleans: &HashSet<String>, written_if: bool) -> Expr {
    // Of branches alike, the value alone, where the test does nothing:
    // `value._0` of `value.TAG === "Continue" ? value._0 : value._0`. One
    // that does something keeps its `if` (`alike_branches`).
    if alike_branches(&yes, &no) && !test.has_effects() {
        return yes;
    }
    // Where the test itself is the value, of what's not a `bool`, `!!c`;
    // its negation too, `!!o` of `!o`'s, `is_some()`'s.
    let truth = |test: Expr| match is_boolean(&test, booleans) {
        true => test,
        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, test)),
    };
    // The test's negation, as a `bool`: a chain's, `a || b` of `!a && !b`,
    // each part a `bool` still; `!!a` of a value's `!a`, whose negation
    // alone is the value.
    let negate = |test: Expr| {
        truth(match test.kind {
            ExprKind::Binary(Op::And | Op::Or, ..) => negated_bools(test, booleans),
            _ => test.negated_test(),
        })
    };
    // `!x ? y : x`, and `x ? x : y`, of an `x` that does nothing:
    // `x || y`, which reads it once (ADR 0744); `x != null ? x : y`, and `x ==
    // null ? y : x`, an `Option`'s value or another's, `x ?? y` (ADR 0298).
    let coalesced =
        |set, x: &Expr| !test.has_effects() && js::null_test(&test, set).is_some_and(|t| js::same_path(t, x));
    match &test.kind {
        ExprKind::Unary(UnaryOp::Not, x) if !x.has_effects() && js::same_path(x, &no) => {
            return Expr::bin(Op::Or, no, yes);
        }
        _ if !test.has_effects() && js::same_path(&test, &yes) => return Expr::bin(Op::Or, yes, no),
        _ if coalesced(true, &yes) => return Expr::bin(Op::Coalesce, yes, no),
        _ if coalesced(false, &no) => return Expr::bin(Op::Coalesce, no, yes),
        _ => {}
    }
    // A written `if`'s branch of a `bool`, its test one too.
    let bool_of = |branch: &Expr| written_if && is_boolean(&test, booleans) && is_boolean(branch, booleans);
    match (&yes.kind, &no.kind) {
        (ExprKind::Bool(true), ExprKind::Bool(false)) => truth(test),
        (ExprKind::Bool(false), ExprKind::Bool(true)) => negate(test),
        (ExprKind::Bool(true), _) if bool_of(&no) => Expr::bin(Op::Or, test, no),
        (_, ExprKind::Bool(false)) if bool_of(&yes) => Expr::bin(Op::And, test, yes),
        _ if written_if => Expr::cond(test, yes, no),
        (ExprKind::Bool(true), _) => Expr::bin(Op::Or, truth(test), no),
        (ExprKind::Bool(false), _) => Expr::bin(Op::And, negate(test), no),
        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, negate(test), yes),
        (_, ExprKind::Bool(false)) => Expr::bin(Op::And, truth(test), yes),
        _ => Expr::cond(test, yes, no),
    }
}
