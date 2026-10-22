//! A `match` of constants the program wrote, its arms' chain of tests
//! settled: a `switch`, as JS writes one, `switch (tag) { case
//! FunctionComponent: .. }` (ADR 0629). Which `match` is one is
//! `hints::written_switches`'; how its arms' tests read, its chain's.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp};
use crate::lower::mir::hints::{Constant, Switch, SwitchArm};

use super::{first_written, is_temporary, relabeled, written_loop};

/// What a readable label a loop's given is, where a `break` of it in a
/// `switch` names it and the program named it none: `loop`, as ADR 0015
/// names one.
const LOOP_LABEL: &str = "loop";

/// Each chain of `body` that tests the arms of a `match` of `switches`, in
/// the order written, a `switch`; a `break` in it of a loop around it, of
/// that loop's label, the program's of `loops` where it wrote one.
pub(super) fn switches(body: &mut Vec<Stmt>, switches: &[Switch], loops: &[(js::Span, bool, Option<String>)]) {
    if switches.is_empty() {
        return;
    }
    walk(body, switches, loops, &mut Vec::new(), None);
}

/// The loop a `switch` is in: the label a `break` of it names, and whether
/// one does.
struct Around<'l> {
    label: &'l str,
    named: bool,
}

/// `stmts`' chains, each a `switch`, then what's in their statements';
/// `around`, the loop they're in, and `labels`, the labels of the loops
/// they're in.
fn walk(
    list: &mut Vec<Stmt>,
    switches: &[Switch],
    loops: &[(js::Span, bool, Option<String>)],
    labels: &mut Vec<String>,
    mut around: Option<&mut Around>,
) {
    let mut i = 0;
    while i < list.len() {
        if let Some((switch, taken)) = switch_at(list, i, switches, around.as_deref_mut()) {
            list.splice(i..i + taken, [switch]);
            // A temporary only its test reads, `const _5 = s.charCodeAt(i);`
            // just before, but for what's declared, unset, `let v;`: its
            // value, read in place.
            let declared = (list[..i].iter().rev())
                .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
                .count();
            if let Some(at) = i.checked_sub(declared + 1)
                && let StmtKind::Const(name, value) = &list[at].kind
                && is_temporary(name)
                && let StmtKind::Switch { discriminant, .. } = &list[i].kind
                && matches!(&discriminant.kind, ExprKind::Var(v) if v == name)
                && js::mentions_in(&list[at + 1..], name) == 1
            {
                let value = value.clone();
                if let StmtKind::Switch { discriminant, .. } = &mut list[i].kind {
                    *discriminant = Expr {
                        span: discriminant.span,
                        ..value
                    };
                }
                list.remove(at);
                i -= 1;
            }
            named_value(list, i);
        }
        i += 1;
    }
    for stmt in list.iter_mut() {
        let span = stmt.span;
        match &mut stmt.kind {
            StmtKind::If(_, a, b) => {
                walk(a, switches, loops, labels, around.as_deref_mut());
                if let Some(b) = b {
                    walk(b, switches, loops, labels, around.as_deref_mut());
                }
            }
            StmtKind::Labeled(_, body) => walk(body, switches, loops, labels, around.as_deref_mut()),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                walk(a, switches, loops, labels, around.as_deref_mut());
                walk(b, switches, loops, labels, around.as_deref_mut());
            }
            StmtKind::Switch { cases, default, .. } => {
                for body in js::case_lists_mut(cases, default) {
                    walk(body, switches, loops, labels, around.as_deref_mut());
                }
            }
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => {
                // Its label, a readable one where MIR made it, `bb5`: the
                // program's, or `loop`, apart from the labels around it and
                // in it.
                let readable = match label.as_deref() {
                    Some(l) if !crate::lower::mir::is_block_label(l) => l.to_string(),
                    _ => {
                        let mut taken: HashSet<String> = labels.iter().cloned().collect();
                        labels_in(body, &mut taken);
                        let written = written_loop(loops, span).and_then(|(.., l)| l.clone());
                        (written.into_iter())
                            .chain((0..).map(|n| match n {
                                0 => LOOP_LABEL.to_string(),
                                n => format!("{LOOP_LABEL}${n}"),
                            }))
                            .find(|name| !taken.contains(name))
                            .expect("a name no label has")
                    }
                };
                let mut inner = Around {
                    label: &readable,
                    named: false,
                };
                labels.push(readable.clone());
                walk(body, switches, loops, labels, Some(&mut inner));
                labels.pop();
                // A case's `continue` of the loop whose turn the `switch`
                // ends, which it goes on to anyway.
                if let Some(Stmt {
                    kind: StmtKind::Switch { cases, default, .. },
                    ..
                }) = body.last_mut()
                {
                    for case in js::case_lists_mut(cases, default) {
                        if matches!(case.last().map(|s| &s.kind), Some(StmtKind::Continue(l))
                            if l.is_none() || l.as_deref() == label.as_deref())
                        {
                            case.pop();
                        }
                    }
                }
                if inner.named {
                    if let Some(from) = label.as_deref()
                        && from != readable
                    {
                        relabeled(body, from, &readable);
                    }
                    *label = Some(readable);
                }
            }
            _ => {}
        }
    }
}

/// A temporary each case of the `switch` at `list[i]` gives a value, read
/// once, just after, by the name the program gives it, `let tmp; switch
/// (..) { case 1: tmp = a; .. } const slice = tmp;`: that name, `let slice;
/// .. slice = a;`, as an `if`'s branches' value is its `?:`'s.
fn named_value(list: &mut Vec<Stmt>, i: usize) {
    let Some(Stmt {
        kind: StmtKind::Const(name, read),
        ..
    }) = list.get(i + 1)
    else {
        return;
    };
    let ExprKind::Var(temporary) = &read.kind else { return };
    let declared = (list[..i].iter().rev())
        .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
        .position(|s| matches!(&s.kind, StmtKind::Let(t, None) if t == temporary));
    let Some(at) = declared.map(|k| i - 1 - k) else { return };
    // Each of its mentions in the `switch` a case's assignment of it.
    let mut assigned = 0;
    js::statement_lists(&mut vec![list[i].clone()], &mut |stmts| {
        assigned += (stmts.iter())
            .filter(
                |s| matches!(&s.kind, StmtKind::Assign(t, _) if matches!(&t.kind, ExprKind::Var(v) if v == temporary)),
            )
            .count();
    });
    let in_switch = js::mentions_in(&list[i..=i], temporary);
    if !is_temporary(temporary) || assigned != in_switch || js::mentions_in(list, temporary) != in_switch + 1 {
        return;
    }
    let (temporary, name) = (temporary.clone(), name.clone());
    js::each_expr_mut(&mut list[i..=i], &mut |e| {
        if let ExprKind::Var(v) = &mut e.kind
            && *v == temporary
        {
            *v = name.clone();
        }
    });
    list.remove(i + 1);
    list[at].kind = StmtKind::Let(name, None);
}

/// A case of a `switch`: its values, its statements, whether it's the
/// `default`, a last `_`'s, and where its arm is written.
type Case = (Vec<Expr>, Vec<Stmt>, bool, js::Span);

/// The `switch` of the chain at `stmts[at]`, of the `match` whose first
/// arm's pattern its test is written at, and how many statements it is.
/// `around`, the loop it's in, whose label each `break` of it names now.
fn switch_at(stmts: &[Stmt], at: usize, switches: &[Switch], around: Option<&mut Around>) -> Option<(Stmt, usize)> {
    let first = &stmts[at];
    let at_test = tested_at(first)?;
    let written = (switches.iter()).find(|s| s.arms.first().is_some_and(|arm| within(at_test, arm.pattern)))?;
    let arms = &written.arms;
    let mut discriminant = None;
    let (mut cases, mut taken) = chain(&stmts[at..], arms, false, &mut discriminant)?;
    let discriminant = discriminant?;
    // What follows it to its list's end, where one case alone goes on to
    // it and it's that arm's, written there or MIR's: that case's, `'.' =>
    // break` of a loop's `break` after it.
    let after = &stmts[at + taken..];
    let going: Vec<usize> = (0..cases.len())
        .filter(|&c| !cases[c].1.last().is_some_and(js::ends))
        .collect();
    // Each arm a case: one that does nothing, left out, goes on too.
    if let [only] = going[..]
        && cases.len() == arms.len()
        && !cases[only].2
        && !after.is_empty()
        && (after.iter()).all(|s| written_at(s).is_none_or(|at| within(at, cases[only].3)))
    {
        cases[only].1.extend(after.iter().cloned());
        taken = stmts.len() - at;
    }
    // A `break` of no label in a case, of the loop around it, which in a
    // `switch` would leave the `switch`: of the loop's label.
    if cases.iter().any(|(_, body, ..)| js::breaks(body, None)) {
        let around = around?;
        for (_, body, ..) in cases.iter_mut() {
            labeled_breaks(body, around.label);
        }
        around.named = true;
    }
    let mut switched = Vec::new();
    let mut default = None;
    for (values, body, last, _) in cases {
        match last {
            true => default = Some(body),
            false => switched.push((values, body)),
        }
    }
    // Where the `match` is written, not its first arm's pattern, which the
    // first case's constant is (ADR 0630).
    let span = written.span;
    let switch = StmtKind::Switch {
        discriminant,
        cases: switched,
        default,
    };
    Some((switch.at(span), taken))
}

/// The cases `stmts` test of `arms`, in order, and how many statements they
/// are: an `if` of each arm's test, its `else` the rest's, or, where its
/// branch leaves, the next; what's left the last arm's, all of `stmts`
/// where they're all the `match`'s, `whole`, as an `else` is, else what's
/// written in it. And of an arm before the last, an `if` of its test
/// negated, the last arm's branch, `if (c !== ".") { .. }`, as a `match`
/// whose arm does nothing, or leaves, MIR tests. `discriminant`, what each
/// test compares, as the first tells.
fn chain(
    stmts: &[Stmt],
    arms: &[SwitchArm],
    whole: bool,
    discriminant: &mut Option<Expr>,
) -> Option<(Vec<Case>, usize)> {
    let mut cases = Vec::new();
    let mut i = 0;
    for (k, arm) in arms.iter().enumerate() {
        let tested = match stmts.get(i) {
            Some(
                stmt @ Stmt {
                    kind: StmtKind::If(test, then, otherwise),
                    ..
                },
            ) if !arm.constants.is_empty() && tested_at(stmt).is_some_and(|at| within(at, arm.pattern)) => {
                Some((test, then, otherwise))
            }
            _ => None,
        };
        let Some((test, then, otherwise)) = tested else {
            // The rest, untested: the last arm's.
            if k + 1 != arms.len() {
                return None;
            }
            let taken = rest(&stmts[i..], arm, whole);
            let values = untested(arm)?;
            cases.push((values, stmts[i..i + taken].to_vec(), arm.constants.is_empty(), arm.span));
            return Some((cases, i + taken));
        };
        // Its test negated, the last arm's branch: its own the `else`, or
        // what's after where that leaves.
        let mut negated = discriminant.clone();
        if k + 2 == arms.len()
            && let Some(values) = values_of(test, &arm.constants, true, &mut negated)
        {
            *discriminant = negated;
            let last = &arms[k + 1];
            let (own, taken) = match otherwise {
                Some(otherwise) => (otherwise.clone(), 0),
                None if then.last().is_some_and(js::ends) => {
                    let taken = rest(&stmts[i + 1..], arm, whole);
                    (stmts[i + 1..i + 1 + taken].to_vec(), taken)
                }
                None => (Vec::new(), 0),
            };
            cases.push((values, own, false, arm.span));
            cases.push((untested(last)?, then.clone(), last.constants.is_empty(), last.span));
            let taken = i + 1 + taken;
            return (!whole || taken == stmts.len()).then_some((cases, taken));
        }
        cases.push((
            values_of(test, &arm.constants, false, discriminant)?,
            then.clone(),
            false,
            arm.span,
        ));
        i += 1;
        match otherwise {
            // The rest, the `else`'s, all of it.
            Some(otherwise) => {
                let (rest, taken) = chain(otherwise, &arms[k + 1..], true, discriminant)?;
                if taken != otherwise.len() {
                    return None;
                }
                cases.extend(rest);
                return Some((cases, i));
            }
            // The next, where this one leaves.
            None if then.last().is_some_and(js::ends) => {}
            // The arms after it do nothing.
            None => return Some((cases, i)),
        }
    }
    // Each arm tested: what's after them isn't the `match`'s, but in an
    // `else`, which nothing reaches.
    (!whole || i == stmts.len()).then_some((cases, i))
}

/// How many of `stmts`, after the arms tested before `arm`, are its: all,
/// where they're all the `match`'s, `whole`, else those written in it.
fn rest(stmts: &[Stmt], arm: &SwitchArm, whole: bool) -> usize {
    match whole {
        true => stmts.len(),
        false => (stmts.iter())
            .take_while(|s| written_at(s).is_none_or(|at| within(at, arm.span)))
            .count(),
    }
}

/// The values of `arm`, which no test says: none of a `_`, the `default`;
/// a variant's, of a last arm Rust proved the rest of.
fn untested(arm: &SwitchArm) -> Option<Vec<Expr>> {
    match arm.constants.as_slice() {
        [] => Some(Vec::new()),
        [Constant::Variant(value)] => Some(vec![value.clone()]),
        _ => None,
    }
}

/// The values `test` compares what it tests to, of an arm of `constants`,
/// each in turn: `n === 1 || n === 2`'s, or, `negated`, `n !== 1 && n !==
/// 2`'s, of what `discriminant` is, or the first says, `n`.
fn values_of(test: &Expr, constants: &[Constant], negated: bool, discriminant: &mut Option<Expr>) -> Option<Vec<Expr>> {
    let (joined, compares) = match negated {
        false => (Op::Or, Op::Eq),
        true => (Op::And, Op::Ne),
    };
    let mut tests = Vec::new();
    alternatives(test, joined, &mut tests);
    if tests.len() != constants.len() {
        return None;
    }
    let mut values = Vec::new();
    for (test, constant) in tests.into_iter().zip(constants) {
        let ExprKind::Binary(op, a, b) = &test.kind else {
            return None;
        };
        if *op != compares {
            return None;
        }
        // Of `n === 1` or `1 === n`, the value the other's compared to.
        let (tested, value) = match &*discriminant {
            Some(d) if alike(d, b) => (b, a),
            Some(_) => (a, b),
            None if literal(a) && !literal(b) => (b, a),
            None => (a, b),
        };
        let (fits, written) = match constant {
            Constant::Variant(variant) => (alike(variant, value), js::Span::NONE),
            Constant::Value(span) => (true, *span),
        };
        if !fits || !compared(discriminant, tested) {
            return None;
        }
        // Where its pattern's constant is written, the test's being MIR's.
        let mut value = (**value).clone();
        if !written.is_none() {
            value.span = written;
        }
        values.push(value);
    }
    Some(values)
}

/// Whether `tested` is what `discriminant` is, or the first is.
fn compared(discriminant: &mut Option<Expr>, tested: &Expr) -> bool {
    match discriminant {
        Some(d) => alike(d, tested),
        None => {
            *discriminant = Some(tested.clone());
            true
        }
    }
}

/// Each test `a || b || c` is of, in order, of `joined`, `||` or `&&`.
fn alternatives<'e>(test: &'e Expr, joined: Op, tests: &mut Vec<&'e Expr>) {
    match &test.kind {
        ExprKind::Binary(op, a, b) if *op == joined => {
            alternatives(a, joined, tests);
            alternatives(b, joined, tests);
        }
        _ => tests.push(test),
    }
}

/// Whether `e` is a literal: a number, a string, a `bool`.
fn literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Num(_) | ExprKind::BigInt(_) | ExprKind::BigUint(_) | ExprKind::Str(_) | ExprKind::Bool(_) => true,
        ExprKind::Unary(UnaryOp::Neg, n) => literal(n),
        _ => false,
    }
}

/// Whether `a` and `b` are the same JS, wherever each was written.
fn alike(a: &Expr, b: &Expr) -> bool {
    let unspanned = |e: &Expr| {
        let mut e = e.clone();
        e.each_mut(&mut |e| e.span = js::Span::NONE);
        e
    };
    unspanned(a) == unspanned(b)
}

/// Where an `if`'s test is written: its arm's pattern, by its test's span,
/// else the `if`'s.
fn tested_at(stmt: &Stmt) -> Option<u32> {
    let StmtKind::If(test, ..) = &stmt.kind else {
        return None;
    };
    test_at(test, stmt.span)
}

/// Where `test`, of an `if` at `span`, is written.
fn test_at(test: &Expr, span: js::Span) -> Option<u32> {
    [test.span, span].into_iter().find(|s| !s.is_none()).map(|s| s.lo)
}

/// Whether `if (test)`, at `span`, tests an arm of a `match` of constants,
/// a `switch`, whose arms' values are no `?:` then (ADR 0629).
pub(super) fn switched(test: &Expr, span: js::Span, switches: &[Switch]) -> bool {
    let arm = |at: u32| {
        (switches.iter())
            .flat_map(|s| &s.arms)
            .any(|arm| within(at, arm.pattern))
    };
    test_at(test, span).is_some_and(arm)
}

/// Where `stmt` is first written: what it does, or the value it gives.
fn written_at(stmt: &Stmt) -> Option<u32> {
    first_written(std::slice::from_ref(stmt)).or_else(|| match &stmt.kind {
        StmtKind::Return(Some(e)) | StmtKind::Throw(e) | StmtKind::Expr(e) if !e.span.is_none() => Some(e.span.lo),
        _ => None,
    })
}

/// Whether `at` is in `span`.
fn within(at: u32, span: js::Span) -> bool {
    !span.is_none() && span.lo <= at && at < span.hi
}

/// Each `break` of no label in `stmts`, not in a loop of their own, of
/// `label`: the loop's around the `switch` they're a case of.
fn labeled_breaks(stmts: &mut [Stmt], label: &str) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Break(l @ None) => *l = Some(label.to_string()),
            StmtKind::If(_, a, b) => {
                labeled_breaks(a, label);
                if let Some(b) = b {
                    labeled_breaks(b, label);
                }
            }
            StmtKind::Labeled(_, body) => labeled_breaks(body, label),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                labeled_breaks(a, label);
                labeled_breaks(b, label);
            }
            _ => {}
        }
    }
}

/// Each label in `stmts`, a loop's or a block's.
fn labels_in(stmts: &[Stmt], labels: &mut HashSet<String>) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => {
                labels.extend(label.clone());
                labels_in(body, labels);
            }
            StmtKind::Labeled(label, body) => {
                labels.insert(label.clone());
                labels_in(body, labels);
            }
            StmtKind::If(_, a, b) => {
                labels_in(a, labels);
                labels_in(b.as_deref().unwrap_or_default(), labels);
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                labels_in(a, labels);
                labels_in(b, labels);
            }
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists(cases, default).for_each(|body| labels_in(body, labels));
            }
            _ => {}
        }
    }
}
