//! Tidy's branches: tests and chains of them, a value each branch gives,
//! and a test as a person writes it.

use super::*;

/// A `match` reading a table by its key (ADR 0233), as MIR tests each
/// variant: `if (k === "a") { v = M.a; } else { v = k === "b" ? M.b : M.c; }`,
/// `v = M[k]`; of each arm its own name, `"a"`, `v = k`. And of returns,
/// `if (k === "a") { return M.a; } .. return M.c;`, `return M[k];`.
pub(super) fn table_reads(stmts: &mut Vec<Stmt>, tables: &[(js::Span, Vec<String>)]) {
    // The `match` written where `s` is, innermost, and its variants' names.
    let table_of = |s: js::Span| {
        (tables.iter())
            .filter(|(t, _)| !s.is_none() && t.lo <= s.lo && s.hi <= t.hi)
            .min_by_key(|(t, _)| t.hi - t.lo)
            .map(|(_, names)| names.as_slice())
    };
    let within = |s: js::Span| table_of(s).is_some();
    let mut i = 0;
    while i < stmts.len() {
        let returned = (stmts[i..].iter())
            .take_while(|s| {
                within(s.span)
                    && matches!(&s.kind, StmtKind::If(_, then, None)
                        if matches!(then.as_slice(), [Stmt { kind: StmtKind::Return(Some(_)), .. }]))
            })
            .count();
        // The last a `return`, or one more test's, `if (k === "b") { return
        // M.b; } else { return M.c; }`, as a `switch`'s arms leave (ADR 0629).
        let (tested, last) = match stmts.get(i + returned).map(|s| &s.kind) {
            Some(StmtKind::Return(Some(last))) => (None, Some(last)),
            Some(StmtKind::If(test, then, Some(otherwise))) => match (then.as_slice(), otherwise.as_slice()) {
                (
                    [
                        Stmt {
                            kind: StmtKind::Return(Some(value)),
                            ..
                        },
                    ],
                    [
                        Stmt {
                            kind: StmtKind::Return(Some(last)),
                            ..
                        },
                    ],
                ) => (Some((Some(test.clone()), value.clone())), Some(last)),
                _ => (None, None),
            },
            _ => (None, None),
        };
        if (returned > 0 || tested.is_some())
            && let Some(last) = last
        {
            let mut arms: Vec<(Option<Expr>, Expr)> = (stmts[i..i + returned].iter())
                .map(|s| match &s.kind {
                    StmtKind::If(test, then, None) => match then.as_slice() {
                        [
                            Stmt {
                                kind: StmtKind::Return(Some(value)),
                                ..
                            },
                        ] => (Some(test.clone()), value.clone()),
                        _ => unreachable!("taken"),
                    },
                    _ => unreachable!("taken"),
                })
                .collect();
            arms.extend(tested);
            value_arms(last, &mut arms);
            if let Some(read) = table_of(stmts[i].span).and_then(|names| table_read(&arms, names)) {
                let span = stmts[i].span;
                stmts.splice(i..=i + returned, [StmtKind::Return(Some(read)).at(span)]);
            }
        }
        i += 1;
    }
    for stmt in stmts.iter_mut() {
        if !within(stmt.span) {
            continue;
        }
        let mut arms = Vec::new();
        let target = match &stmt.kind {
            StmtKind::If(..) => assigned_arms(stmt, &mut arms),
            // Taken apart too, `const { name, classes } = ..`, as react.dev's
            // Tag takes its variant.
            StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Return(Some(e))
            | StmtKind::Destructure { value: e, .. } => {
                value_arms(e, &mut arms);
                None
            }
            StmtKind::Assign(target, e) => {
                value_arms(e, &mut arms);
                Some(target.clone())
            }
            _ => continue,
        };
        let Some(read) = table_of(stmt.span).and_then(|names| table_read(&arms, names)) else {
            continue;
        };
        match (&mut stmt.kind, target) {
            (StmtKind::If(..), Some(target)) => stmt.kind = StmtKind::Assign(target, read),
            (
                StmtKind::Const(_, e)
                | StmtKind::Let(_, Some(e))
                | StmtKind::Return(Some(e))
                | StmtKind::Assign(_, e)
                | StmtKind::Destructure { value: e, .. },
                _,
            ) => *e = read,
            _ => {}
        }
    }
}

/// The arms of `if (k === "a") { v = .. } else ..`, each its test and value,
/// and what each assigns, the same.
pub(super) fn assigned_arms(stmt: &Stmt, arms: &mut Vec<(Option<Expr>, Expr)>) -> Option<Expr> {
    match &stmt.kind {
        StmtKind::If(test, then, Some(otherwise)) => {
            let [
                Stmt {
                    kind: StmtKind::Assign(target, value),
                    ..
                },
            ] = then.as_slice()
            else {
                return None;
            };
            arms.push((Some(test.clone()), value.clone()));
            let [rest] = otherwise.as_slice() else { return None };
            let rest = assigned_arms(rest, arms)?;
            js::same_path(&rest, target).then_some(rest)
        }
        StmtKind::Assign(target, value) => {
            value_arms(value, arms);
            Some(target.clone())
        }
        _ => None,
    }
}

/// The arms of `k === "a" ? M.a : ..`, each its test and value.
pub(super) fn value_arms(e: &Expr, arms: &mut Vec<(Option<Expr>, Expr)>) {
    match &e.kind {
        ExprKind::Cond(test, value, rest) => {
            arms.push((Some((**test).clone()), (**value).clone()));
            value_arms(rest, arms);
        }
        _ => arms.push((None, e.clone())),
    }
}

/// `M[k]`, of arms each `k === "a"` giving `M.a`, the last whichever's
/// left, of `names`, the variants'; or `k` itself, of each giving its own
/// name.
pub(super) fn table_read(arms: &[(Option<Expr>, Expr)], names: &[String]) -> Option<Expr> {
    let [first, .., (None, _)] = arms else { return None };
    let (Some(first), _) = first else { return None };
    let ExprKind::Binary(Op::Eq, key, _) = &first.kind else {
        return None;
    };
    // The one variant the last arm is of: the one no test names.
    let tested: Vec<&str> = (arms.iter())
        .filter_map(|(test, _)| match &test.as_ref()?.kind {
            ExprKind::Binary(Op::Eq, _, name) => match &name.kind {
                ExprKind::Str(name) => Some(name.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let [left] = (names.iter())
        .filter(|n| !tested.contains(&n.as_str()))
        .collect::<Vec<_>>()[..]
    else {
        return None;
    };
    let mut table: Option<&Expr> = None;
    let mut fields = Vec::new();
    let mut names = 0;
    for (test, value) in arms {
        let name = match test {
            Some(test) => {
                let ExprKind::Binary(Op::Eq, k, name) = &test.kind else {
                    return None;
                };
                let ExprKind::Str(name) = &name.kind else { return None };
                if !js::same_path(k, key) {
                    return None;
                }
                Some(name.as_str())
            }
            None => Some(left.as_str()),
        };
        match &value.kind {
            ExprKind::Member(of, field) if name.is_none_or(|n| n == field) && !fields.contains(&field.as_str()) => {
                if table.is_some_and(|t| !js::same_path(t, of)) {
                    return None;
                }
                table = Some(of);
                fields.push(field.as_str());
            }
            ExprKind::Str(s) if name.is_none_or(|n| n == s) => names += 1,
            _ => return None,
        }
    }
    match table {
        None if names == arms.len() => Some((**key).clone()),
        Some(table) if fields.len() == arms.len() => Some(Expr::index(table.clone(), (**key).clone())),
        _ => None,
    }
}

/// What a branch makes a value of, as a person writes it: `if (c) {} else
/// { s }` is `if (!c) { s }`; `if (c) { x = a; } else { x = b; }` is `x =
/// c ? a : b;`, of two branches; and `l: { if (a) { if (b) { x = t; break l; } } x = f; }`,
/// `matches!`'s, is `x = a && b ? t : f;`.
pub(super) fn branch_values(
    stmts: &mut Vec<Stmt>,
    returns: &[js::Span],
    booleans: &HashSet<String>,
    (result, switches): (&str, &[Switch]),
    program: &crate::lower::mir::hints::Written,
) {
    let (continues, elses) = (&program.continues, &program.elses);
    stmts.retain_mut(|stmt| {
        let mut span = stmt.span;
        let kind = std::mem::replace(&mut stmt.kind, StmtKind::Break(None));
        // A branch with nothing left, as `emptied_branch` writes it.
        let Some(kind) = emptied_branch(kind) else {
            return false;
        };
        stmt.kind = match kind {
            // `if (a) { break l; } else if (b) { break l; }`, an or-pattern's
            // tests: `if (a || b) { break l; }`.
            StmtKind::If(a, then, Some(mut otherwise))
                if let [
                    Stmt {
                        kind: StmtKind::Break(Some(l)),
                        ..
                    },
                ] = then.as_slice()
                    && let [
                        Stmt {
                            kind: StmtKind::If(_, inner, None),
                            ..
                        },
                    ] = otherwise.as_slice()
                    && matches!(inner.as_slice(), [Stmt { kind: StmtKind::Break(Some(m)), .. }] if m == l) =>
            {
                let Some(Stmt {
                    kind: StmtKind::If(b, _, None),
                    ..
                }) = otherwise.pop()
                else {
                    unreachable!("matched")
                };
                StmtKind::If(or_all(a, b), then, None)
            }
            // A chain of more arms, its last two folded first, `if (a) { x = p; }
            // else { x = b ? q : r; }`: the chain, as it's written; of a
            // temporary, where its first arm does more than give it, so the
            // whole is no conditional either.
            StmtKind::If(ref test, ref then, Some(ref otherwise))
                if let Some(Stmt {
                    kind: StmtKind::Assign(target, _),
                    ..
                }) = then.last()
                    && let ExprKind::Var(x) = &target.kind
                    && (!is_temporary(x) && then.len() == 1 || is_temporary(x) && then.len() > 1)
                    && let Some((y, folded)) = assigned_only(otherwise)
                    && x == y
                    && !switched(test, span, switches)
                    && let ExprKind::Cond(t, q, r) = &folded.kind
                    && small(q)
                    && small(r) =>
            {
                let at = otherwise[0].span;
                let arm = |value: &Expr| vec![StmtKind::Assign(Expr::var(y), value.clone()).at(at)];
                let chained = StmtKind::If((**t).clone(), arm(q), Some(arm(r))).at(at);
                StmtKind::If(test.clone(), then.clone(), Some(vec![chained]))
            }
            StmtKind::If(test, ref then, Some(ref otherwise))
                if let Some(one) = one_assignment(&test, (then, otherwise), span, (result, switches)) =>
            {
                // A value the program wrote with an `if`, not `&&` or `||`.
                let written_if = written_branches(span, then, otherwise, elses);
                let value = match one.no_first {
                    true => conditional(test.negated_test(), one.no, one.yes, booleans, written_if),
                    false => conditional(test, one.yes, one.no, booleans, written_if),
                };
                StmtKind::Assign(Expr::var(&one.target), value)
            }
            // `l: { if (!a) { if (!b) { s; break l; } } t }`, `||`'s: `if (a ||
            // b) { t } else { s }`, where nothing else breaks to `l`; and `l:
            // { if (a) { if (b) { s; break l; } } t }`, `&&`'s, `if (a && b) {
            // s } else { t }`: the branch the program wrote first, first, its
            // test as written (ADR 0683).
            StmtKind::Labeled(label, body) if let Some((tests, otherwise, then)) = or_chain(&body, &label) => {
                span = made_where(span, &body);
                let test = (tests.into_iter())
                    .rev()
                    .reduce(|b, a| Expr::bin(Op::And, a, b))
                    .expect("one");
                let chain_first =
                    matches!((first_written(&otherwise), first_written(&then)), (Some(s), Some(t)) if s < t);
                match (then.is_empty(), chain_first) {
                    (true, _) => StmtKind::If(test, otherwise, None),
                    (false, true) => StmtKind::If(test, otherwise, Some(then)),
                    (false, false) => StmtKind::If(test.negated_test(), then, Some(otherwise)),
                }
            }
            StmtKind::Labeled(label, body) => match chain(&body, &label) {
                Some((tests, x, yes, no)) => {
                    let test = (tests.into_iter())
                        .rev()
                        .reduce(|b, a| Expr::bin(Op::And, a, b))
                        .expect("one");
                    StmtKind::Assign(Expr::var(&x), conditional(test, yes, no, booleans, false))
                }
                None => StmtKind::Labeled(label, body),
            },
            kind => kind,
        };
        stmt.span = span;
        true
    });
    stmts.retain(|stmt| !matches!(&stmt.kind, StmtKind::Expr(e) if matches!(e.kind, ExprKind::Undefined)));
    guarded_chains(stmts);
    // `if (a) { if (b) { return t; } } return f;`, last: `return a && b ? t : f;`.
    if let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(_, then, _) = &last_if.kind
        && let Some((tests, yes)) = returned_chain(std::slice::from_ref(last_if))
        && small(&yes)
        && small(no)
        && boolean_branch(&yes, no)
        // Not where the program wrote `return`, an early one.
        && !written(returns, last.span)
        && !writes_return(std::slice::from_ref(last_if), returns)
    {
        let test = (tests.into_iter())
            .rev()
            .reduce(|b, a| Expr::bin(Op::And, a, b))
            .expect("one");
        // Of an `if` the program wrote with an `else`, as it wrote it.
        let written_if = written_branches(last_if.span, then, std::slice::from_ref(last), elses)
            || written_values(last_if.span, (&yes, no), elses);
        let value = conditional(test, yes, no.clone(), booleans, written_if);
        let span = last_if.span;
        stmts.truncate(stmts.len() - 2);
        stmts.push(StmtKind::Return(Some(value)).at(span));
    } else if let [before @ .., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(_)) = &last.kind
        && written(returns, last.span)
        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if))
        // Not `if c { return a; } else { return b; }`'s: a `let … else`'s
        // rest ends where it goes on, not at a `return` the program wrote;
        // nor `if c { continue; } return b;`'s, an `if` the program wrote
        // of a `continue` (ADR 0684).
        && !body.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(_)) && written(returns, s.span))
        && !tests_continue(continues, last_if.span)
        && !declared(&body).any(|name| declared(before).any(|n| n == name))
    {
        // `f` the program wrote, a `let … else`'s else: `if (!(a && b)) {
        // return f; } t`.
        let test = (tests.into_iter())
            .rev()
            .reduce(|b, a| Expr::bin(Op::And, a, b))
            .expect("one");
        let (span, guard) = (last_if.span, stmts.pop().expect("the return"));
        stmts.pop();
        stmts.push(StmtKind::If(test.negated_test(), vec![guard], None).at(span));
        stmts.extend(body);
    }
}

/// Of `if (a) { const x = e; if (b) { s } }`, `s` ending where it leaves:
/// `[a, b]` and `const x = e; s`, what a level binds before its test, a
/// read that does nothing, written into later tests, as `broken_chain`'s:
/// each kept, so not a property, which a test would read again
/// (`bound_into`).
pub(super) fn returning_chain(stmts: &[Stmt]) -> Option<(Vec<Expr>, Vec<Stmt>)> {
    let [
        bound @ ..,
        Stmt {
            kind: StmtKind::If(test, inner, None),
            ..
        },
    ] = stmts
    else {
        return None;
    };
    let (mut tests, body) = match returning_chain(inner) {
        Some(chain) => chain,
        None if inner.last().is_some_and(ends) => (Vec::new(), inner.clone()),
        None => return None,
    };
    tests.insert(0, test.clone());
    for test in &mut tests {
        *test = bound_into(test, bound, &|_, _| 1)?;
    }
    let mut all = bound.to_vec();
    all.extend(body);
    unread_bound(&mut all, bound.len());
    Some((tests, all))
}

/// Whether of `c ? yes : no` one is `true` or `false`: `c || no` or `c &&
/// yes`, not `?:`.
pub(super) fn boolean_branch(yes: &Expr, no: &Expr) -> bool {
    matches!(yes.kind, ExprKind::Bool(_)) || matches!(no.kind, ExprKind::Bool(_))
}

/// Of `if (a) { if (b) { return t; } }`: its tests, `[a, b]`, and `t`.
pub(super) fn returned_chain(stmts: &[Stmt]) -> Option<(Vec<Expr>, Expr)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::Return(Some(yes)),
                ..
            },
        ] => Some((Vec::new(), yes.clone())),
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, yes) = returned_chain(inner)?;
            tests.insert(0, test.clone());
            Some((tests, yes))
        }
        _ => None,
    }
}

/// `value`, which a branch of a conditional is, where its statement was,
/// `at`, of one with no place of its own, `(f(a) + f(b)) >>> 0`: the source
/// map points each branch at what it's of, not its neighbor's, as it
/// pointed the statement.
pub(super) fn spanned(value: &Expr, at: js::Span) -> Expr {
    let mut value = value.clone();
    // The printer maps a name or a literal where it starts, not a binary, a
    // call or a member: its first token, however deep, starts it, mapped
    // to the nearest of what it's in that's placed, `fib(n - 1)` of `fib`.
    let mut placed = at;
    let mut first = &mut value;
    loop {
        if !first.span.is_none() {
            placed = first.span;
        }
        first = match &mut first.kind {
            ExprKind::Binary(_, left, _) | ExprKind::Call(left, _) | ExprKind::Member(left, _) => left,
            _ => break,
        };
    }
    if first.span.is_none() {
        first.span = placed;
    }
    value
}

/// Whether the function's value of two branches, `yes` and `no`, is one
/// `return t ? yes : no`: not where one gives nothing, `undefined`, whose
/// guard, `if (t) { return yes; }`, reads as one.
pub(super) fn folds_into_return(yes: &Expr, no: &Expr) -> bool {
    [yes, no].iter().all(|value| !matches!(value.kind, ExprKind::Undefined))
}

/// Whether `e` reads well as a branch of `c ? a : b`: not a conditional
/// itself, of more branches, nor an element, which reads best returned or
/// assigned as it is. A function is one value, `c ? f : (x) => { .. }`, as
/// React writes one: what a branch does before its value is what keeps it
/// a statement (ADR 0657).
pub(super) fn small(e: &Expr) -> bool {
    // An element of two levels at most, `<b>Save</b>`, a value; one of more
    // reads best as a branch of its own.
    fn levels(e: &Expr) -> usize {
        match &e.kind {
            ExprKind::Jsx(jsx) => 1 + jsx.children.iter().map(levels).max().unwrap_or(0),
            _ => 0,
        }
    }
    match &e.kind {
        ExprKind::Jsx(_) => levels(e) <= 2,
        _ => !matches!(e.kind, ExprKind::Cond(..)),
    }
}

/// Whether `value`, given `x`, is `x` changed, `x + a`: `x += a`.
pub(super) fn updates(x: &str, value: &Expr) -> bool {
    matches!(&value.kind, ExprKind::Binary(_, of, _) if matches!(&of.kind, ExprKind::Var(v) if v == x))
}

/// `if (test) { x = yes; } else { x = no; }` as one assignment, `x = test ?
/// yes : no;`: what it assigns, each arm's value placed where its arm is,
/// and whether the `else`'s comes first. The one place that says whether
/// two arms are one value (`branch_values` writes what it says).
pub(super) struct OneAssignment {
    pub(super) target: String,
    pub(super) yes: Expr,
    pub(super) no: Expr,
    pub(super) no_first: bool,
}

/// Whether the arms of an `if` of `test`, at `span`, are one assignment,
/// and how: both assign one variable alone; not of more branches, which
/// read best as they are, but where it's a temporary's, an `if` the
/// program wrote as a value; the function's result only where it folds
/// into its `return` (`returned_values`); not each adding to it, `out +=
/// a`, as written; nor a `switch`'s arms (ADR 0629). The value the program
/// wrote first comes first: `a || b ? x : y` of `if a || b { x } else { y }`
/// MIR tests `!a && !b` of, and a chain's next test in the `else`
/// whichever comes first, as a match's arms are written.
pub(super) fn one_assignment(
    test: &Expr,
    (then, otherwise): (&[Stmt], &[Stmt]),
    span: js::Span,
    (result, switches): (&str, &[Switch]),
) -> Option<OneAssignment> {
    let (x, yes) = assigned_only(then)?;
    let (y, no) = assigned_only(otherwise)?;
    let one = x == y
        && ((small(yes) && small(no)) || is_temporary(x))
        && (x != result || folds_into_return(yes, no))
        && !(updates(x, yes) && updates(x, no))
        && !switched(test, span, switches);
    if !one {
        return None;
    }
    let no_first = match (
        matches!(yes.kind, ExprKind::Cond(..)),
        matches!(no.kind, ExprKind::Cond(..)),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => matches!((first_written(then), first_written(otherwise)), (Some(t), Some(o)) if o < t),
    };
    Some(OneAssignment {
        target: x.to_string(),
        yes: spanned(yes, then[0].span),
        no: spanned(no, otherwise[0].span),
        no_first,
    })
}

/// The variable `stmts` only assign, and what to: `x = a;`.
pub(super) fn assigned_only(stmts: &[Stmt]) -> Option<(&str, &Expr)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::Assign(target, value),
                ..
            },
        ] => match &target.kind {
            ExprKind::Var(x) => Some((x.as_str(), value)),
            _ => None,
        },
        _ => None,
    }
}

/// Of `l: { if (a) { if (b) { x = t; break l; } } x = f; }`: its tests,
/// `[a, b]`, `x`, `t` and `f`.
pub(super) fn chain(body: &[Stmt], label: &str) -> Option<(Vec<Expr>, String, Expr, Expr)> {
    let [
        Stmt {
            kind: StmtKind::If(test, inner, None),
            ..
        },
        last,
    ] = body
    else {
        return None;
    };
    let (x, no) = assigned_only(std::slice::from_ref(last))?;
    let (mut tests, y, yes) = inner_chain(inner, label)?;
    if x != y {
        return None;
    }
    tests.insert(0, test.clone());
    Some((tests, x.to_string(), yes, no.clone()))
}

/// `!e` of a chain of tests, part by part, `!!a || b !== c` of `!a && b ===
/// c`: a `bool`, as `!e` is, each `!x` of what's not one `!!x`.
pub(super) fn negated_bools(e: Expr, booleans: &HashSet<String>) -> Expr {
    match e.kind {
        ExprKind::Binary(op @ (Op::And | Op::Or), left, right) => {
            let flipped = if op == Op::And { Op::Or } else { Op::And };
            Expr::bin(flipped, negated_bools(*left, booleans), negated_bools(*right, booleans))
        }
        ExprKind::Unary(UnaryOp::Not, x) if is_boolean(&x, booleans) => *x,
        ExprKind::Unary(UnaryOp::Not, x) => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, *x)),
        kind => Expr::unary(
            UnaryOp::Not,
            Expr {
                known: js::Known::Nothing,
                kind,
                span: e.span,
            },
        ),
    }
}

/// `else { if (t) { throw e; } s }`, a check MIR wrote first in an `else if`
/// chain's last `else`: `else if (!t) { s } else { throw e; }`, the chain as
/// written.
pub(super) fn chained_checks(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::If(_, _, Some(otherwise)) = &mut stmt.kind else {
            continue;
        };
        let [check, _, ..] = otherwise.as_slice() else {
            continue;
        };
        if !matches!(&check.kind, StmtKind::If(_, then, None)
            if matches!(then.as_slice(), [Stmt { kind: StmtKind::Throw(_), .. }]))
        {
            continue;
        }
        let rest = otherwise.split_off(1);
        let check = otherwise.remove(0);
        let StmtKind::If(test, throw, None) = check.kind else {
            unreachable!("matched")
        };
        *otherwise = vec![StmtKind::If(test.negated_test(), rest, Some(throw)).at(check.span)];
    }
}

/// `if (t) { if (..) .. } else { y }`, `y` written before the chain in
/// the `then`: `if (!t) { y } else if (..) ..`, a match's arms as written,
/// which MIR tests `!== "a"` of with the rest first.
pub(super) fn written_chains(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::If(_, then, Some(otherwise)) = &stmt.kind else {
            continue;
        };
        let chain = |list: &[Stmt]| {
            matches!(
                list,
                [Stmt {
                    kind: StmtKind::If(..),
                    ..
                }]
            )
        };
        // Where the chain's first arm is written: its `if` is where the match is.
        let arm = match then.as_slice() {
            [
                Stmt {
                    kind: StmtKind::If(_, arm, _),
                    ..
                },
            ] => first_written(arm),
            _ => None,
        };
        let earlier = matches!((first_written(otherwise), arm), (Some(o), Some(t)) if o < t);
        if !chain(then) || chain(otherwise) || !earlier {
            continue;
        }
        let StmtKind::If(test, then, Some(otherwise)) = std::mem::replace(&mut stmt.kind, StmtKind::Break(None)) else {
            unreachable!("matched")
        };
        stmt.kind = StmtKind::If(test.negated_test(), otherwise, Some(then));
    }
}

/// `t && <div className={t ? e : undefined} />`: `e`, where `t` holds, of a
/// test that does nothing and reads what nothing changes.
pub(super) fn implied_tests(e: &mut Expr, changing: &HashSet<String>) {
    let (ExprKind::Binary(Op::And, test, child) | ExprKind::Cond(test, child, _)) = &mut e.kind else {
        return;
    };
    let mut unchanged = !test.has_effects();
    test.visit_vars(&mut |v| unchanged &= !changing.contains(v));
    if !unchanged {
        return;
    }
    let test = (**test).clone();
    child.each_mut(&mut |e| {
        if let ExprKind::Cond(t, yes, no) = &e.kind
            && matches!(no.kind, ExprKind::Undefined)
            && same_test(t, &test)
        {
            *e = (**yes).clone();
        }
    });
}

/// Whether `a` and `b` are one test of what nothing changes: one path,
/// `!` of one, or one compared to one constant, `error != null`.
pub(super) fn same_test(a: &Expr, b: &Expr) -> bool {
    // `!!x`, a test of `x`'s truth, as `x` is.
    fn truth(e: &Expr) -> &Expr {
        match &e.kind {
            ExprKind::Unary(UnaryOp::Not, x) if let ExprKind::Unary(UnaryOp::Not, x) = &x.kind => x,
            _ => e,
        }
    }
    let (a, b) = (truth(a), truth(b));
    match (&a.kind, &b.kind) {
        (ExprKind::Unary(UnaryOp::Not, x), ExprKind::Unary(UnaryOp::Not, y)) => same_test(x, y),
        (ExprKind::Binary(p, x, k), ExprKind::Binary(q, y, l)) if p == q => {
            js::same_path(x, y)
                && match (&k.kind, &l.kind) {
                    // `== null` is `== undefined`; `=== null` isn't
                    // `=== undefined` (ADR 0676).
                    (ExprKind::Null | ExprKind::Undefined, ExprKind::Null | ExprKind::Undefined) => {
                        matches!(p, Op::LooseEq | Op::LooseNe) || k.kind == l.kind
                    }
                    (ExprKind::Str(k), ExprKind::Str(l)) => k == l,
                    _ => k.as_int().is_some() && k.as_int() == l.as_int(),
                }
        }
        _ => js::same_path(a, b),
    }
}

/// `const result = e; return result;`, the place MIR makes what a function
/// gives in, `returned`'s once it's a `const`: `return e;`.
pub(super) fn returned_consts(stmts: &mut Vec<Stmt>, result: &str) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let returns = matches!((&stmts[i].kind, &stmts[i + 1].kind), (StmtKind::Const(name, _), StmtKind::Return(Some(value)))
            if name == result && matches!(&value.kind, ExprKind::Var(v) if v == name));
        if returns {
            let StmtKind::Const(_, value) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts[i].kind = StmtKind::Return(Some(value));
        }
        i += 1;
    }
}

/// `a || b`, of `b` a chain itself too, `a || b1 || b2`, as one is written.
pub(super) fn or_all(a: Expr, b: Expr) -> Expr {
    match b.kind {
        ExprKind::Binary(Op::Or, x, y) => Expr::bin(Op::Or, or_all(a, *x), *y),
        kind => Expr::bin(
            Op::Or,
            a,
            Expr {
                known: js::Known::Nothing,
                kind,
                span: b.span,
            },
        ),
    }
}

/// Of `if (b) { x = t; break l; }`, nested: its tests, `x` and `t`.
pub(super) fn inner_chain(stmts: &[Stmt], label: &str) -> Option<(Vec<Expr>, String, Expr)> {
    match stmts {
        [
            assign,
            Stmt {
                kind: StmtKind::Break(Some(l)),
                ..
            },
        ] if l == label => {
            let (x, yes) = assigned_only(std::slice::from_ref(assign))?;
            Some((Vec::new(), x.to_string(), yes.clone()))
        }
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, x, yes) = inner_chain(inner, label)?;
            tests.insert(0, test.clone());
            Some((tests, x, yes))
        }
        _ => None,
    }
}

/// `test ? yes : no`: `test && yes` where `no` is `false`, and `test || no`
/// where `yes` is `true`, the test `!!test` where it's not a `bool`; but of
/// an `if` the program wrote with an `else`, `written_if`, `!c ? x : false`,
/// as it wrote it, not the `&&` MIR makes the same branches of (ADR 0879).
pub(super) fn conditional(test: Expr, yes: Expr, no: Expr, booleans: &HashSet<String>, written_if: bool) -> Expr {
    // Where the test itself is the value, of what's not a `bool`, `!!c`;
    // its negation too, `!!o` of `!o`'s, `is_some()`'s.
    let truth = |test: Expr| match is_boolean(&test, booleans) {
        true => test,
        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, test)),
    };
    // A chain's negation, `a || b` of `!a && !b`, each part a `bool` still,
    // `!!a` of a value's `!a`.
    let negate = |test: Expr| match test.kind {
        ExprKind::Binary(Op::And | Op::Or, ..) => negated_bools(test, booleans),
        _ => test.negated_test(),
    };
    // `!x ? y : x`, and `x ? x : y`, `!!x`'s too, of an `x` that does nothing:
    // `x || y`, which reads it once (ADR 0744); `x != null ? x : y`, and `x ==
    // null ? y : x`, an `Option`'s value or another's, `x ?? y` (ADR 0298).
    let coalesced =
        |set, x: &Expr| !test.has_effects() && js::null_test(&test, set).is_some_and(|t| js::same_path(t, x));
    match &test.kind {
        ExprKind::Unary(UnaryOp::Not, x) if !x.has_effects() && js::same_path(x, &no) => {
            return Expr::bin(Op::Or, no, yes);
        }
        _ if !test.has_effects() && js::same_path(&test.tested(), &yes) => return Expr::bin(Op::Or, yes, no),
        _ if coalesced(true, &yes) => return Expr::bin(Op::Coalesce, yes, no),
        _ if coalesced(false, &no) => return Expr::bin(Op::Coalesce, no, yes),
        _ => {}
    }
    match (&yes.kind, &no.kind) {
        (ExprKind::Bool(true), ExprKind::Bool(false)) => truth(test),
        (ExprKind::Bool(false), ExprKind::Bool(true)) => truth(negate(test)),
        _ if written_if => Expr::cond(test, yes, no),
        (ExprKind::Bool(true), _) => Expr::bin(Op::Or, truth(test), no),
        (ExprKind::Bool(false), _) => Expr::bin(Op::And, truth(negate(test)), no),
        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, truth(negate(test)), yes),
        (_, ExprKind::Bool(false)) => Expr::bin(Op::And, truth(test), yes),
        _ => Expr::cond(test, yes, no),
    }
}

/// Whether `test` holds where `by` does: `x != null` where `x === "lit"`,
/// `typeof x === "string"` there too, and where each side of `a || b` says
/// it.
pub(super) fn implies(by: &Expr, test: &Expr) -> bool {
    match (&test.kind, &by.kind) {
        (_, ExprKind::Binary(Op::Or, a, b)) => implies(a, test) && implies(b, test),
        // `x != null` where `x != null`.
        (ExprKind::Binary(Op::LooseNe, x, null), ExprKind::Binary(Op::LooseNe, y, also))
            if [null, also]
                .iter()
                .all(|n| matches!(n.kind, ExprKind::Null | ExprKind::Undefined)) =>
        {
            js::same_path(x, y)
        }
        // `x != null` where `typeof x === "number"`: of any kind but
        // `object`, `null`'s, and `undefined`.
        (ExprKind::Binary(Op::LooseNe, x, null), ExprKind::Binary(Op::Eq, kind, name))
            if matches!(null.kind, ExprKind::Null | ExprKind::Undefined)
                && let ExprKind::Unary(UnaryOp::Typeof, y) = &kind.kind =>
        {
            js::same_path(x, y) && matches!(&name.kind, ExprKind::Str(t) if t != "object" && t != "undefined")
        }
        (ExprKind::Binary(Op::LooseNe, x, null), ExprKind::Binary(Op::Eq, y, literal))
            if matches!(null.kind, ExprKind::Null | ExprKind::Undefined) =>
        {
            js::same_path(x, y)
                && literal.is_constant()
                && !matches!(literal.kind, ExprKind::Null | ExprKind::Undefined)
        }
        (ExprKind::Binary(Op::Eq, kind, name), ExprKind::Binary(Op::Eq, y, literal))
            if let ExprKind::Unary(UnaryOp::Typeof, x) = &kind.kind =>
        {
            js::same_path(x, y)
                && match (&name.kind, &literal.kind) {
                    (ExprKind::Str(t), ExprKind::Str(_)) => t == "string",
                    (ExprKind::Str(t), ExprKind::Num(_)) => t == "number",
                    (ExprKind::Str(t), ExprKind::Bool(_)) => t == "boolean",
                    _ => false,
                }
        }
        // `x != null` where `x` holds, or `!!x`: `null` and `undefined` are
        // falsy (ADR 0870).
        (ExprKind::Binary(Op::LooseNe, x, null), _)
            if matches!(null.kind, ExprKind::Null | ExprKind::Undefined) && js::same_path(x, &by.tested()) =>
        {
            true
        }
        _ => false,
    }
}

/// `if (x != null) { if (x === "a") { s } else if (x === "b") { t } }`,
/// and `if (typeof x === "string") { .. }` of the same chain, which no
/// string literal's `===` needs: the chains alone, which say it (ADR 0755).
pub(super) fn guarded_chains(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        match &stmts[i].kind {
            StmtKind::If(test, then, None) if !then.is_empty() && then.iter().all(|s| guarded_chain(s, test)) => {
                let StmtKind::If(_, then, _) = stmts.remove(i).kind else {
                    unreachable!("matched")
                };
                let n = then.len();
                stmts.splice(i..i, then);
                i += n;
            }
            _ => i += 1,
        }
    }
}

/// Whether each test of the `else if` chain `stmt` says `guard`, and it has
/// no last `else`: where `guard` fails, it does nothing.
pub(super) fn guarded_chain(stmt: &Stmt, guard: &Expr) -> bool {
    match &stmt.kind {
        StmtKind::If(test, _, otherwise) => {
            implies(test, guard)
                && (otherwise.as_deref()).is_none_or(|rest| matches!(rest, [next] if guarded_chain(next, guard)))
        }
        _ => false,
    }
}

/// JS's own methods, of an array, an iterator, a map, a set or a string,
/// whose answer is a `bool`, as rust-js writes them of std's.
const BOOLEAN_METHODS: &[&str] = &["some", "every", "includes", "has", "startsWith", "endsWith"];

/// Whether `e` is a `bool` whatever it reads: a comparison, a `!`, one of
/// `booleans`, one of JS's methods whose answer is one, or `&&` and `||`
/// of them.
pub(super) fn is_boolean(e: &Expr, booleans: &HashSet<String>) -> bool {
    match &e.kind {
        ExprKind::Bool(_) | ExprKind::Unary(UnaryOp::Not, _) => true,
        ExprKind::Var(name) => booleans.contains(name),
        ExprKind::Member(..) => field_path(e).is_some_and(|path| booleans.contains(&path)),
        ExprKind::Call(..) if call_path(e).is_some_and(|path| booleans.contains(&path)) => true,
        ExprKind::Call(callee, _) if matches!(&callee.kind, ExprKind::Member(_, name) if BOOLEAN_METHODS.contains(&name.as_str())) => {
            true
        }
        ExprKind::Binary(
            Op::Eq | Op::Ne | Op::LooseEq | Op::LooseNe | Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::InstanceOf | Op::In,
            ..,
        ) => true,
        ExprKind::Binary(Op::And | Op::Or, a, b) => is_boolean(a, booleans) && is_boolean(b, booleans),
        _ => false,
    }
}

/// `!(b >= 48)`, a comparison of what's never `NaN`: `b < 48`, as a person
/// writes it. An integer is never `NaN`, nor what a bitwise operator or a
/// `length` gives; a string compared, to a string literal as Rust's types
/// say, is ordered too. A float may be `NaN`, neither less nor more (ADR
/// 0371).
pub(super) fn turned(e: &mut Expr, numbers: &HashSet<String>) {
    let ExprKind::Unary(UnaryOp::Not, inner) = &e.kind else {
        return;
    };
    let ExprKind::Binary(op, a, b) = &inner.kind else {
        return;
    };
    let op = match op {
        Op::Lt => Op::Ge,
        Op::Le => Op::Gt,
        Op::Gt => Op::Le,
        Op::Ge => Op::Lt,
        _ => return,
    };
    let never_nan = |x: &Expr| match &x.kind {
        ExprKind::Var(v) => numbers.contains(v),
        ExprKind::Num(n) => n.fract() == 0.0,
        ExprKind::Binary(Op::BitOr | Op::BitAnd | Op::BitXor | Op::Shl | Op::Shr | Op::UShr, ..) => true,
        ExprKind::Member(_, m) => m == "length",
        _ => false,
    };
    let text = |x: &Expr| matches!(x.kind, ExprKind::Str(_));
    if text(a) || text(b) || (never_nan(a) && never_nan(b)) {
        *e = Expr {
            span: e.span,
            ..Expr::bin(op, (**a).clone(), (**b).clone())
        };
    }
}

/// `if (c) { s; result = a; } else { result = b; } return result;`, the
/// function's value its branches make: returned where each is made, `if (c)
/// { s; return a; } return b;`, at the `return`s the program wrote. Of a
/// temporary it's made in before it's the result, too, a `Box<dyn Fn()>`'s
/// before it's coerced, `_5 = work; .. return _5;` (ADR 0599).
pub(super) fn returned_in_branches(stmts: &mut Vec<Stmt>, result: &str) {
    returned_from_block(stmts, result);
    let [.., last_if, last] = stmts.as_slice() else {
        return;
    };
    let StmtKind::Return(Some(returned)) = &last.kind else {
        return;
    };
    let ExprKind::Var(returned) = &returned.kind else {
        return;
    };
    // Not a variable the program names, `let item;`, which it returns.
    if returned != result && !is_temporary(returned) {
        return;
    }
    let result = returned.as_str();
    if !matches!(last_if.kind, StmtKind::If(_, _, Some(_))) || !made_at_ends(std::slice::from_ref(last_if), result) {
        return;
    }
    let mut made = stmts[stmts.len() - 2].clone();
    returned_at_ends(std::slice::from_mut(&mut made));
    // Read nowhere else.
    if js::mentions_in(std::slice::from_ref(&made), result) > 0
        || js::mentions_in(&stmts[..stmts.len() - 2], result) > 0
    {
        return;
    }
    stmts.truncate(stmts.len() - 2);
    // An `else` after a branch that returns: what follows it.
    match made.kind {
        StmtKind::If(test, then, Some(otherwise)) if then.last().is_some_and(ends) => {
            stmts.push(StmtKind::If(test, then, None).at(made.span));
            stmts.extend(otherwise);
        }
        kind => stmts.push(Stmt { kind, span: made.span }),
    }
}

/// What a `filter` by truth keeps, of `x`: `x || undefined`, as lowering
/// writes it (ADR 0635), or `x ? x : undefined`, a test tidied to `x`'s
/// truth, `x != null && x !== ""` of text's.
fn kept_by_truth(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        ExprKind::Binary(Op::Or, x, no) if matches!(no.kind, ExprKind::Undefined) => Some(x),
        ExprKind::Cond(test, x, no) if matches!(no.kind, ExprKind::Undefined) && js::same_path(&test.tested(), x) => {
            Some(x)
        }
        _ => None,
    }
}

/// `const h = x || undefined; if (h == null) { exit }`, a `let … else`
/// of what `filter` keeps: `if (!x) { exit }`, `h` and
/// what binds it then `x`, which it is; and `if (h != null) { s }`, `h`
/// read there alone, `if (x) { s }`.
pub(super) fn filtered_guards(stmts: &mut Vec<Stmt>, changing: &HashSet<String>) {
    for i in 1..stmts.len() {
        let [made, checked] = &stmts[i - 1..=i] else {
            unreachable!("two")
        };
        let StmtKind::Const(h, value) = &made.kind else {
            continue;
        };
        let Some(ExprKind::Var(x)) = kept_by_truth(value).map(|x| &x.kind) else {
            continue;
        };
        if changing.contains(x) {
            continue;
        }
        let StmtKind::If(guard, then, otherwise) = &checked.kind else {
            continue;
        };
        let ExprKind::Binary(op @ (Op::LooseEq | Op::LooseNe), tested, null) = &guard.kind else {
            continue;
        };
        if !matches!(&tested.kind, ExprKind::Var(t) if t == h)
            || !matches!(null.kind, ExprKind::Null | ExprKind::Undefined)
        {
            continue;
        }
        let kept = match op {
            Op::LooseEq => otherwise.is_none() && then.last().is_some_and(ends),
            _ => {
                otherwise.as_deref().is_none_or(|o| js::mentions_in(o, h) == 0)
                    && js::mentions_in(&stmts[i + 1..], h) == 0
            }
        };
        if !kept {
            continue;
        }
        let truth = match op {
            Op::LooseEq => Expr::unary(UnaryOp::Not, Expr::var(x)),
            _ => Expr::var(x),
        };
        // `h`, read as `x`: it goes.
        let (h, x) = (h.clone(), x.clone());
        stmts.remove(i - 1);
        let mut rest = stmts.split_off(i - 1);
        let StmtKind::If(guard, ..) = &mut rest[0].kind else {
            unreachable!("the test, where the `const` was")
        };
        *guard = truth;
        js::each_expr_mut(&mut rest, &mut |e| {
            if matches!(&e.kind, ExprKind::Var(v) if *v == h) {
                *e = Expr {
                    span: e.span,
                    ..Expr::var(&x)
                };
            }
        });
        stmts.extend(rest);
        return;
    }
}

/// Of `if (b) { if (c) { s } }`, the body of an `if`: its tests, `[b, c]`,
/// and `s`; of others, none and themselves.
pub(super) fn ands(stmts: &[Stmt]) -> (Vec<Expr>, Vec<Stmt>) {
    match stmts {
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, body) = ands(inner);
            tests.insert(0, test.clone());
            (tests, body)
        }
        _ => (Vec::new(), stmts.to_vec()),
    }
}

/// An or-pattern of constants as MIR tests it, each alternative failing, one
/// inside another, `if (s !== "a") { if (s !== "e") { b } } t`, where `b`
/// and `t` leave: its arm first, `if (s === "a" || s === "e") { t } b`.
pub(super) fn or_tests(stmts: &mut Vec<Stmt>) {
    // Each test, `s !== "a"`, of one subject, and what the last guards.
    fn chain<'s>(stmt: &'s Stmt, subject: &mut Option<&'s Expr>, tests: &mut Vec<&'s Expr>) -> Option<&'s [Stmt]> {
        let StmtKind::If(test, then, None) = &stmt.kind else {
            return None;
        };
        let ExprKind::Binary(Op::Ne, of, constant) = &test.kind else {
            return None;
        };
        if !constant.is_constant() || !subject.is_none_or(|s| js::same_path(s, of)) {
            return None;
        }
        *subject = Some(of);
        tests.push(test);
        match then.as_slice() {
            [inner] if let Some(body) = chain(inner, subject, tests) => Some(body),
            body => Some(body),
        }
    }
    let mut i = 0;
    while i + 1 < stmts.len() {
        let mut tests = Vec::new();
        let made = chain(&stmts[i], &mut None, &mut tests).and_then(|body| {
            let rest = &stmts[i + 1..];
            (tests.len() > 1
                && body.last().is_some_and(ends)
                && rest.last().is_some_and(ends)
                && !declared(body).any(|name| declared(rest).any(|n| n == name)))
            .then(|| {
                let test = (tests.iter())
                    .map(|t| (*t).clone().negated_test())
                    .reduce(|a, b| Expr::bin(Op::Or, a, b))
                    .expect("two");
                (test, body.to_vec())
            })
        });
        if let Some((test, body)) = made {
            let span = stmts[i].span;
            let rest: Vec<Stmt> = stmts.drain(i + 1..).collect();
            stmts[i] = StmtKind::If(test, rest, None).at(span);
            stmts.extend(body);
            return;
        }
        i += 1;
    }
}

/// A test an earlier one decided, of what nothing changes between: in `if
/// (t) { a } else { b }`, `t` holds through `a` and fails through `b`, and
/// where `a` leaves, fails through what follows the `if` (where `b` leaves,
/// holds); so an `if (t)` there is its `then`, or its `else`, and an `if
/// (!t)` the other way. A std body's drop of what its early return gave
/// away, `if (option != null) { return option; } .. if (option != null) {
/// drop }`, and a drop's test inside its own (ADR 0564). `t` of what nothing
/// assigns, `changing`'s; one that reads through a field or a helper,
/// `x.value`, `$someValue(x)`, only up to the next call or assignment
/// through a field, which may change it. Not an `if` the program wrote, of
/// a condition in `written`: what it says (ADR 0564, amended).
pub(super) fn decided_tests(stmts: &mut Vec<Stmt>, changing: &HashSet<String>, written: &[js::Span]) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::If(test, then, otherwise) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        if !decidable(test, changing) {
            i += 1;
            continue;
        }
        let test = test.clone();
        let then_leaves = then.last().is_some_and(ends);
        let else_leaves = otherwise.as_ref().is_some_and(|o| o.last().is_some_and(ends));
        let StmtKind::If(_, then, otherwise) = &mut stmts[i].kind else {
            unreachable!("matched")
        };
        decided_in(then, 0, (&test, written), true, true);
        if let Some(otherwise) = otherwise {
            decided_in(otherwise, 0, (&test, written), false, true);
        }
        match (then_leaves, else_leaves) {
            (true, false) => decided_in(stmts, i + 1, (&test, written), false, false),
            (false, true) => decided_in(stmts, i + 1, (&test, written), true, false),
            _ => {}
        }
        i += 1;
    }
}

/// Whether `test` can be decided by an earlier reading of it: it does
/// nothing, reads no variable something assigns, and isn't a constant.
fn decidable(test: &Expr, changing: &HashSet<String>) -> bool {
    let mut stable = !test.has_effects() && !test.is_constant();
    test.visit_vars(&mut |v| stable &= !changing.contains(v));
    stable
}

/// `stmts` from `from` on, where `test` is known to hold, or to fail: each
/// `if` of it, or of its negation, its branch taken, up to what may change
/// what `test` reads; and, in a branch of the `if` of `test`, its `parts`,
/// each part of a chain it decides left out (ADR 0743). Not past the `if`,
/// where a `match`'s arms are tested one after another, each test whole,
/// which a `switch` of them reads (ADR 0668).
fn decided_in(stmts: &mut Vec<Stmt>, from: usize, (test, written): (&Expr, &[js::Span]), holds: bool, parts: bool) {
    let wrote = |t: &Expr| !t.span.is_none() && written.iter().any(|c| c.lo < t.span.hi && t.span.lo < c.hi);
    let mut j = from;
    while j < stmts.len() {
        match &mut stmts[j].kind {
            StmtKind::If(t, ..) if disturbs(t, test) => return,
            StmtKind::If(t, ..) if let Some(taken) = decided(t, test).filter(|_| !wrote(t)) => {
                let StmtKind::If(_, then, otherwise) = stmts.remove(j).kind else {
                    unreachable!("matched")
                };
                let branch = match taken == holds {
                    true => then,
                    false => otherwise.unwrap_or_default(),
                };
                stmts.splice(j..j, branch);
                // What the branch had is where it was, known the same there.
            }
            StmtKind::If(t, ..) if parts && let Some(folded) = decided_part(t, test, holds) => {
                // A part of its chain decided, `a || x` where `x` fails: `a`,
                // looked at again (ADR 0743).
                let StmtKind::If(t, ..) = &mut stmts[j].kind else {
                    unreachable!("matched")
                };
                *t = folded;
            }
            StmtKind::If(_, then, otherwise) => {
                decided_in(then, 0, (test, written), holds, parts);
                let mut disturbed = disturbs_any(then, test);
                if let Some(otherwise) = otherwise {
                    decided_in(otherwise, 0, (test, written), holds, parts);
                    disturbed |= disturbs_any(otherwise, test);
                }
                if disturbed {
                    return;
                }
                j += 1;
            }
            StmtKind::Labeled(_, body) => {
                decided_in(body, 0, (test, written), holds, parts);
                if disturbs_any(body, test) {
                    return;
                }
                j += 1;
            }
            _ => match disturbs_any(std::slice::from_ref(&stmts[j]), test) {
                true => return,
                false => {
                    present_in(&mut stmts[j], test, holds);
                    j += 1;
                }
            },
        }
    }
}

/// Of `stmt`, where `test` is known to hold, or to fail, each `x ?? d` of
/// a variable it says is there, `typeof x === "function"` holding or `x ==
/// null` failing: `x`, which `d` never replaces (ADR 0620).
fn present_in(stmt: &mut Stmt, test: &Expr, holds: bool) {
    let known = match holds {
        true => test.clone(),
        false => test.clone().negated_test(),
    };
    js::each_expr_mut(std::slice::from_mut(stmt), &mut |e| {
        if let ExprKind::Binary(Op::Coalesce, x, _) = &e.kind
            && matches!(x.kind, ExprKind::Var(_))
            && implies(&known, &Expr::bin(Op::LooseNe, (**x).clone(), Expr::null()))
        {
            *e = (**x).clone().or_at(e.span);
        }
    });
}

/// Of `t`, a chain of `&&` and `||`, where `test` is known to hold, or to
/// fail: the chain without the parts of it `test` decides, `a ||
/// Array.isArray(v)` in an `else` of `Array.isArray(v)` its `a`; none where
/// no part of it is (ADR 0743). A part whose value is known is left out of
/// its chain only where it does nothing, as a test that's decided does not.
fn decided_part(t: &Expr, test: &Expr, holds: bool) -> Option<Expr> {
    fn fold(e: &Expr, test: &Expr, holds: bool, changed: &mut bool) -> Expr {
        let ExprKind::Binary(op @ (Op::And | Op::Or), a, b) = &e.kind else {
            return match decided(e, test) {
                Some(taken) => {
                    *changed = true;
                    Expr::bool(taken == holds).or_at(e.span)
                }
                None => e.clone(),
            };
        };
        let (a, b) = (fold(a, test, holds, changed), fold(b, test, holds, changed));
        // What `a op b` is where one side is known: `true || b` is `true`,
        // `false || b` is `b`, `a || false` is `a`; `&&` the other way.
        let known = |x: &Expr| match x.kind {
            ExprKind::Bool(v) => Some(v),
            _ => None,
        };
        let short = matches!(op, Op::Or);
        match (known(&a), known(&b)) {
            (Some(v), _) if v == short => a,
            (Some(_), _) => b,
            (_, Some(v)) if v != short => a,
            (_, Some(_)) if !a.has_effects() => b,
            _ => Expr::bin(*op, a, b).or_at(e.span),
        }
    }
    if !matches!(t.kind, ExprKind::Binary(Op::And | Op::Or, ..)) {
        return None;
    }
    let mut changed = false;
    let folded = fold(t, test, holds, &mut changed);
    changed.then_some(folded)
}

/// Whether `t` is `test`, `Some(true)`, or its negation, `Some(false)`.
fn decided(t: &Expr, test: &Expr) -> Option<bool> {
    let same = |a: &Expr, b: &Expr| same_test(a, b) || a.written_alike(b);
    if same(t, test) {
        return Some(true);
    }
    same(&t.clone().negated_test(), test).then_some(false)
}

/// Whether evaluating `e` may change what `test` reads: nothing, of a test
/// that reads only variables nothing assigns; of one that reads through a
/// field or a helper, anything that does something.
fn disturbs(e: &Expr, test: &Expr) -> bool {
    !test.reads_only_vars() && e.has_effects()
}

/// Whether running `stmts` may change what `test` reads: an assignment of a
/// variable it reads; and, of one that reads through a field or a helper,
/// an assignment through a field, or anything that does something.
fn disturbs_any(stmts: &[Stmt], test: &Expr) -> bool {
    let mut vars = Vec::new();
    test.visit_vars(&mut |v| vars.push(v.to_string()));
    if vars.iter().any(|v| assigns(stmts, v)) {
        return true;
    }
    if test.reads_only_vars() {
        return false;
    }
    let mut stmts = stmts.to_vec();
    let mut disturbed = false;
    js::statement_lists(&mut stmts, &mut |list| {
        disturbed |= list.iter().any(|s| {
            matches!(&s.kind, StmtKind::Assign(target, _) | StmtKind::Compound(_, target, _) if !matches!(target.kind, ExprKind::Var(_)))
        });
    });
    js::each_expr_mut(&mut stmts, &mut |e| disturbed |= e.has_effects());
    disturbed
}

/// What both branches of an `if` end with, one statement MIR wrote into
/// each, `if (c) { a; x = 1; } else { b; x = 1; }`: written once, after
/// it, `if (c) { a } else { b } x = 1;`; and of an `if` without `else` whose
/// branch ends as what follows the `if` does, `if (c) { a; return v; }
/// return v;`, the branch's copy goes, `if (c) { a } return v;` (ADR 0564).
/// A tail that leaves, `return v`, stays in its two branches: where each
/// exits is the branches' shape, the guard clause `let_else_first` and
/// `early_returns` make of it; and a branch's copy of what follows goes
/// only of a `return` of a value or a `throw`, a `return;` of nothing or
/// of `undefined` being the function's end `tail_returns` writes as a
/// guard clause, and a `break` or `continue` a loop's, `early_breaks`'. A
/// branch that is only the tail stays, an arm
/// `decisions` merges with the one alike; and one that reads what a branch
/// declares stays in it.
pub(super) fn shared_tails(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::If(_, then, otherwise) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        match otherwise {
            Some(otherwise) => {
                let shared = shared_tail(then, otherwise);
                let (kept_then, kept_else) = (&then[..then.len() - shared], &otherwise[..otherwise.len() - shared]);
                let tail = &then[then.len() - shared..];
                if shared == 0
                    || tail.last().is_some_and(ends)
                    || kept_then.is_empty()
                    || reads_declared(tail, kept_then.iter().chain(kept_else))
                {
                    i += 1;
                    continue;
                }
                let StmtKind::If(_, then, Some(otherwise)) = &mut stmts[i].kind else {
                    unreachable!("matched")
                };
                let tail = then.split_off(then.len() - shared);
                otherwise.truncate(otherwise.len() - shared);
                if otherwise.is_empty() {
                    let StmtKind::If(_, _, otherwise) = &mut stmts[i].kind else {
                        unreachable!("matched")
                    };
                    *otherwise = None;
                }
                stmts.splice(i + 1..i + 1, tail);
            }
            // Of a tail that returns a value, or throws: a `return;`, of
            // nothing or of `undefined`, is the function's end, `tail_returns`'
            // guard clause, and a `break` or `continue` a loop's, `early_breaks`'.
            None if then.last().is_some_and(returns_a_value) => {
                let rest = &stmts[i + 1..];
                let shared = shared_tail(then, rest);
                let kept = &then[..then.len() - shared];
                if shared == 0 || shared != rest.len() || kept.is_empty() || reads_declared(rest, kept.iter()) {
                    i += 1;
                    continue;
                }
                let StmtKind::If(_, then, None) = &mut stmts[i].kind else {
                    unreachable!("matched")
                };
                then.truncate(then.len() - shared);
            }
            None => {}
        }
        // A branch with nothing left, as `emptied_branch` writes it.
        let kind = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None));
        match emptied_branch(kind) {
            Some(kind) => stmts[i].kind = kind,
            None => {
                stmts.remove(i);
                continue;
            }
        }
        i += 1;
    }
}

/// An `if` a branch of which is emptied, as a person writes it: `if (c) {}
/// else { b }` is `if (!c) { b }`, `if (c) { a } else {}` is `if (c) { a }`,
/// and `if (c) {}` of a test that does nothing is nothing, `None`, of one
/// that does, the test alone, `c;`. The one owner of an emptied branch,
/// which each pass that empties one asks.
pub(super) fn emptied_branch(kind: StmtKind) -> Option<StmtKind> {
    let StmtKind::If(test, then, otherwise) = kind else {
        return Some(kind);
    };
    match (then.is_empty(), otherwise) {
        (true, Some(otherwise)) if !otherwise.is_empty() => Some(StmtKind::If(test.negated_test(), otherwise, None)),
        (true, _) if !test.has_effects() => None,
        (true, _) => Some(StmtKind::Expr(test)),
        (false, Some(otherwise)) if otherwise.is_empty() => Some(StmtKind::If(test, then, None)),
        (false, otherwise) => Some(StmtKind::If(test, then, otherwise)),
    }
}

/// Whether `stmt` leaves with the function's answer: a `return` of a value
/// other than `undefined`, or a `throw`.
fn returns_a_value(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(Some(value)) => !matches!(value.kind, ExprKind::Undefined),
        StmtKind::Throw(_) => true,
        _ => false,
    }
}

/// How many statements `a` and `b` end with that do the same.
fn shared_tail(a: &[Stmt], b: &[Stmt]) -> usize {
    (a.iter().rev().zip(b.iter().rev()))
        .take_while(|(x, y)| decisions::same_stmt(x, y))
        .count()
}

/// Whether `tail` reads a name `branches` declare, which it can't be moved
/// out of.
fn reads_declared<'s>(tail: &[Stmt], branches: impl Iterator<Item = &'s Stmt>) -> bool {
    let branches: Vec<Stmt> = branches.cloned().collect();
    declared(&branches).any(|name| js::mentions_in(tail, name) > 0)
}
