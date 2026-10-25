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
    while let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(test, then, None) = &last_if.kind
        && let [
            Stmt {
                kind: StmtKind::Return(Some(yes)),
                span: yes_span,
            },
        ] = then.as_slice()
        && within(last_if.span)
        && !written(returns, last.span)
        && !written(returns, *yes_span)
    {
        let returned = StmtKind::Return(Some(conditional(
            test.clone(),
            yes.clone(),
            no.clone(),
            booleans,
            false,
        )));
        let span = last_if.span;
        stmts.splice(stmts.len() - 2.., [returned.at(span)]);
    }
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
        if read_twice_apart(test, bound, second, &again) {
            continue;
        }
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
        then.extend(body);
        unread_bound(then, bound);
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

/// `l: { if (a) { if (b) { break l; } r } } s`, MIR's of an `if` the
/// program wrote with `||`, `if (!a || b) { s } r`, where `r` and `s` each
/// end, a `break` the program wrote of a block it labeled or a `return`:
/// its test as written, its branch where it's written (ADR 0790). The
/// paths are the same: `!a`'s and `b`'s to `s`, the rest's to `r`.
pub(super) fn or_blocks(stmts: &mut Vec<Stmt>, logicals: &[js::Span]) {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    let mut i = 0;
    while i + 1 < stmts.len() {
        let made = match (&stmts[i].kind, stmts.last()) {
            (StmtKind::Labeled(label, body), Some(last))
                if crate::lower::mir::is_block_label(label) && ends(last) && i + 1 < stmts.len() =>
            {
                match body.as_slice() {
                    [
                        outer @ Stmt {
                            kind: StmtKind::If(a, then, None),
                            ..
                        },
                    ] if within(outer.span) => match then.as_slice() {
                        [
                            Stmt {
                                kind: StmtKind::If(b, jump, None),
                                ..
                            },
                            rest @ ..,
                        ] if matches!(jump.as_slice(), [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == label)
                            && rest.last().is_some_and(ends)
                            && !js::names_jump(rest, label)
                            // What `r` declares, now beside what's before it.
                            && !declared(rest).any(|n| declared(&stmts[..i]).any(|m| m == n)) =>
                        {
                            let test = Expr::bin(Op::Or, a.clone().negated_test(), b.clone());
                            Some((test, outer.span, rest.to_vec()))
                        }
                        _ => None,
                    },
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some((test, span, rest)) = made {
            let after: Vec<Stmt> = stmts.drain(i + 1..).collect();
            stmts[i] = StmtKind::If(test, after, None).at(span);
            stmts.extend(rest);
        }
        i += 1;
    }
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
    // Whether `parts`, joined with `op`, are a negation the program wrote.
    let written = |parts: &[Expr]| {
        let mut spans = Vec::new();
        let within = |outer: js::Span, s: &js::Span| outer.lo <= s.lo && s.hi <= outer.hi;
        parts.iter().all(|part| tests_of(part, &mut spans))
            && (logicals.iter())
                .filter(|&&l| spans.iter().all(|s| within(l, s)))
                .min_by_key(|l| l.hi - l.lo)
                .is_some_and(|&inner| negations.contains(&(inner, op == Op::Or)))
    };
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
/// 0`, is `0` where what it wraps is. And of any integer's, a variable
/// each of whose locals is one, `numbers`, or a wrap, `(n % 2) | 0`,
/// never `NaN` either: `if (lanes)`. Of a test whose truth alone is read,
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

/// Whether `e` is an integer's value, never `NaN`: a variable each of
/// whose locals is one, a whole literal, a wrap, an `i32`'s `(n % 2) | 0`
/// or a `u32`'s `(a + b) >>> 0`, a number of 32 bits, a bitwise
/// operator's; and a sum, difference or product of those, and a remainder
/// of one by a literal not `0`, `n % 2` (ADR 0896, amended).
fn integer_value(e: &Expr, integer: &dyn Fn(&str) -> bool) -> bool {
    match &e.kind {
        ExprKind::Var(name) => integer(name),
        ExprKind::Num(n) => n.fract() == 0.0,
        ExprKind::Binary(Op::BitOr | Op::UShr, _, zero) if zero.as_int() == Some(0) => true,
        _ if bits(e) => true,
        ExprKind::Binary(Op::Add | Op::Sub | Op::Mul, a, b) => integer_value(a, integer) && integer_value(b, integer),
        ExprKind::Binary(Op::Rem, a, b) => integer_value(a, integer) && b.as_int().is_some_and(|d| d != 0),
        ExprKind::Unary(UnaryOp::Neg, a) => integer_value(a, integer),
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
            if b.as_int() == Some(0) && (bits(a) || (!matched(at) && integer_value(a, integer))) =>
        {
            let ne = *op == Op::Ne;
            let bits = match std::mem::replace(&mut **a, Expr::undefined()) {
                Expr {
                    kind: ExprKind::Binary(Op::UShr, x, zero),
                    ..
                } if zero.as_int() == Some(0) && bits(&x) => *x,
                bits => bits,
            };
            let span = test.span;
            *test = match ne {
                true => bits,
                false => Expr::unary(UnaryOp::Not, bits),
            };
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
