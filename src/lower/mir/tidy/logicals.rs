//! Tidy's logicals: a value the program wrote with `&&` and `||`, which MIR
//! made branches of, returned as one value as it was written, and the
//! tests of one condition joined with `&&`.

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
        let returned = StmtKind::Return(Some(conditional(test.clone(), yes.clone(), no.clone(), booleans)));
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
        let test = conditional(test, Expr::bool(true), Expr::bool(false), booleans);
        return Expr::bin(Op::Or, Expr::bin(Op::And, test, p), no);
    }
    conditional(test, yes, no, booleans)
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
    e.known == js::Known::Getter
}

/// `if (a) { if (b) { s } }`, the tests of one condition the program wrote,
/// `a && b`'s: `if (a && b) { s }`. What's bound between them, a read that
/// does nothing, comes first in `s`, and `b` reads its value, as
/// `if let`s chained with `&&` read; one only `b` read, `const captured =
/// child.type` a closure took, is gone: `b` reads it, once. Not where that
/// would read a property twice (`bound_into`): `const url = request.url;
/// if (typeof url === "string") { return url; }` stays, but where its `s`
/// breaks out to a label (`read_once_first`).
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
        let Some(second) = bound_into(second, bound, &again) else {
            first_reads.extend(read_once_first(stmt).map(|read| (i, read)));
            continue;
        };
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
    let defined = Expr::bin(Op::LooseNe, Expr::var(name), Expr::null());
    let mut tests = Vec::new();
    and_parts(second, &mut tests);
    let breaks = matches!(body.last().map(|s| &s.kind), Some(StmtKind::Break(Some(_))));
    let getter = reads_a_getter(value);
    if !getter || others || second.has_effects() || !breaks || !tests.iter().any(|t| implies(t, &defined)) {
        return None;
    }
    let first = StmtKind::Const(name.clone(), Expr::cond(test.clone(), value.clone(), Expr::undefined())).at(read.span);
    stmt.kind = inner.kind.clone();
    Some(first)
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
/// its test as written, its branch where it's written (ADR 0774). The
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
