//! A `match` of constants the program wrote, its arms' chain of tests
//! settled: a `switch`, as JS writes one, `switch (tag) { case
//! FunctionComponent: .. }` (ADR 0629). Which `match` is one is
//! `hints::written_switches`'; how its arms' tests read, its chain's.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp};
use crate::lower::mir::hints::{Constant, SUBJECT, Switch, SwitchArm};
use crate::lower::untagged::TypeofCase;

use super::{first_written, is_temporary, relabeled, unlabeled, written_loop};

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
            list.splice(i..i + taken, switch);
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
    js::each_list(std::slice::from_ref(&list[i]), &mut |stmts| {
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
type Case = (Values, Vec<Stmt>, bool, js::Span);

/// What an arm's case compares: its values; and of a `switch (typeof
/// value)` (ADR 0668), the tests of its variants `typeof` doesn't tell,
/// `Array.isArray(value)`, and those its variants' tests leave out, which
/// an arm of an array tests before the `switch`; and the tests of its
/// variants `typeof` says are another case's, `null`'s `value === null`
/// of `"object"`, an `if` in that case (ADR 0676).
#[derive(Default)]
struct Values {
    values: Vec<Expr>,
    tested: Vec<Expr>,
    excluding: Vec<Expr>,
    inside: Vec<(&'static str, Expr)>,
}

/// The statements of the chain at `stmts[at]`, of the `match` an arm's
/// pattern of which its test is written at, a `switch`, and how many they
/// are. `around`, the loop it's in, whose label each `break` of it names
/// now.
fn switch_at(
    stmts: &[Stmt],
    at: usize,
    switches: &[Switch],
    around: Option<&mut Around>,
) -> Option<(Vec<Stmt>, usize)> {
    let first = &stmts[at];
    let at_test = tested_at(first)?;
    let written = (switches.iter()).find(|s| (s.arms.iter()).any(|arm| within(at_test, arm.pattern)))?;
    let arms: Vec<&SwitchArm> = written.arms.iter().collect();
    let mut discriminant = None;
    let (mut cases, mut taken) = chain(&stmts[at..], &arms, false, &mut discriminant)?;
    let discriminant = discriminant?;
    // Its cases as its arms are written, whatever order MIR tests them in.
    cases.sort_by_key(|case| case.3.lo);
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
    // Each arm of `null`, which `typeof` says is an object, an `if` of its
    // test in `case "object"`; and each of an array, an `if` of its test
    // before the `switch`, and so what the others' leave out.
    nulls_inside(&mut cases)?;
    let before = arrays_before(&mut cases)?;
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
            false => switched.push((values.values, body)),
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
    // Each `if` before it, its `else` the rest, where it goes on.
    let written = (before.into_iter().rev()).fold(vec![switch.at(span)], |rest, (test, body, at)| {
        match body.last().is_some_and(js::ends) {
            true => std::iter::once(StmtKind::If(test, body, None).at(at))
                .chain(rest)
                .collect(),
            false => vec![StmtKind::If(test, body, Some(rest)).at(at)],
        }
    });
    Some((written, taken))
}

/// Of the cases of a `switch (typeof value)` (ADR 0668), each arm of
/// `null`, which `typeof` says is an object (ADR 0676). An arm that
/// takes the objects too, `Null | Object | Array`, is its `case "object"`
/// already, as its variants' owner tells of them together
/// (`untagged::union_parts`); of one alone, an `if` of its test first in the case of the arm that
/// takes them, `case "object": if (value === null) { .. }`, as React
/// writes it. What that case's test leaves out is then told. Of one alone
/// where no arm takes the objects, an `if` before the `switch`, as an
/// array's is; of `Undefined | Null`, what's `== null`, an `if` of it
/// before the `switch`, as an `Option`'s `None` is tested (ADR 0030); of
/// one of `null` and another case, `Null | String`, none.
fn nulls_inside(cases: &mut Vec<Case>) -> Option<()> {
    // Whether `values` has the case `at`, `"object"`.
    let takes = |values: &[Expr], at: &str| (values.iter()).any(|v| matches!(&v.kind, ExprKind::Str(s) if s == at));
    let mut nullish = Vec::new();
    for case in cases.iter_mut() {
        let values = &mut case.0;
        let undefined = matches!(&values.values[..], [v] if matches!(&v.kind, ExprKind::Str(s) if s == "undefined"));
        if let (true, true, [(_, null)]) = (undefined && values.tested.is_empty(), !case.2, &values.inside[..])
            && let ExprKind::Binary(Op::Eq, subject, _) = &null.kind
        {
            let none = Expr::bin(Op::LooseEq, (**subject).clone(), Expr::null());
            values.tested.push(none.or_at(null.span));
            nullish.push(null.clone());
            values.values.clear();
            values.inside.clear();
        }
    }
    for case in cases.iter_mut() {
        case.0.excluding.retain(|e| !nullish.iter().any(|t| alike(t, e)));
    }
    let mut i = 0;
    while i < cases.len() {
        if cases[i].0.inside.is_empty() {
            i += 1;
            continue;
        }
        let only = cases[i].0.values.is_empty() && cases[i].0.tested.is_empty() && !cases[i].2;
        let ([(at, test)], true) = (&cases[i].0.inside[..], only) else {
            return None;
        };
        let (at, test) = (*at, test.clone());
        // No arm takes the objects: an `if` before the `switch`, as an
        // array's is.
        let Some(taker) = (cases.iter()).position(|(values, ..)| takes(&values.values, at)) else {
            cases[i].0.inside.clear();
            cases[i].0.tested.push(test);
            i += 1;
            continue;
        };
        let (_, body, _, span) = cases.remove(i);
        let taker = if taker > i { taker - 1 } else { taker };
        let (values, rest, ..) = &mut cases[taker];
        values.excluding.retain(|e| !alike(&test, e));
        let taken = std::mem::take(rest);
        *rest = match body.last().is_some_and(js::ends) {
            true => std::iter::once(StmtKind::If(test, body, None).at(span))
                .chain(taken)
                .collect(),
            false => vec![StmtKind::If(test, body, Some(taken)).at(span)],
        };
    }
    Some(())
}

/// Of the cases of a `switch (typeof value)` (ADR 0668), each arm of
/// variants `typeof` doesn't tell, an array's, taken out, an `if` of its
/// test, its statements and where it's written: what's before the
/// `switch`. An arm of an array and an object both, `Object | Array`, is
/// one `case "object"` already, as its variants' owner tells of them
/// together (`untagged::union_parts`); of one of what `typeof` doesn't
/// tell and what it does, `String | Array`, none. What the others' tests
/// leave out, `Kind::Object`'s `!Array.isArray(value)`, is what's before,
/// or there's no `switch`: `case "object"` would take an array.
fn arrays_before(cases: &mut Vec<Case>) -> Option<Vec<(Expr, Vec<Stmt>, js::Span)>> {
    let mut before = Vec::new();
    let mut i = 0;
    while i < cases.len() {
        let values = &mut cases[i].0;
        let alone = std::mem::take(&mut values.tested);
        match (alone.is_empty(), values.values.is_empty() && !cases[i].2) {
            (true, _) => i += 1,
            (false, false) => return None,
            (false, true) => {
                let (_, body, _, span) = cases.remove(i);
                let test = (alone.into_iter())
                    .reduce(|a, b| Expr::bin(Op::Or, a, b))
                    .expect("a test");
                before.push((test, body, span));
            }
        }
    }
    let left_out =
        (cases.iter()).all(|(values, ..)| (values.excluding.iter()).all(|e| before.iter().any(|(t, ..)| alike(t, e))));
    left_out.then_some(before)
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
    arms: &[&SwitchArm],
    whole: bool,
    discriminant: &mut Option<Expr>,
) -> Option<(Vec<Case>, usize)> {
    let mut cases = Vec::new();
    let mut i = 0;
    let mut left = arms.to_vec();
    while !left.is_empty() {
        // The arm whose pattern its test is written at, of those left,
        // wherever MIR tests it, an untagged enum's in its variants' order:
        // each tests what no other does.
        let tested = match stmts.get(i) {
            Some(
                stmt @ Stmt {
                    kind: StmtKind::If(test, then, otherwise),
                    ..
                },
            ) => (left.iter())
                .position(|arm| !arm.constants.is_empty() && tested_at(stmt).is_some_and(|at| within(at, arm.pattern)))
                .map(|k| (left.remove(k), test, then, otherwise)),
            _ => None,
        };
        let Some((arm, test, then, otherwise)) = tested else {
            // The rest, untested: the last arm's, the one left. A chain
            // begun past an arm's test leaves that arm and the last, a `_`
            // or one Rust proved, which no test is of: no `switch`.
            let [arm] = left[..] else {
                return None;
            };
            let taken = rest(&stmts[i..], arm, whole);
            let values = untested(arm, discriminant)?;
            cases.push((values, stmts[i..i + taken].to_vec(), arm.constants.is_empty(), arm.span));
            return Some((cases, i + taken));
        };
        // Its test negated, the last arm's branch: its own the `else`, or
        // what's after where that leaves.
        let mut negated = discriminant.clone();
        if let [last] = left[..]
            && let Some(values) = values_of(test, &arm.constants, true, &mut negated)
        {
            *discriminant = negated;
            let (own, taken) = match otherwise {
                Some(otherwise) => (otherwise.clone(), 0),
                None if then.last().is_some_and(js::ends) => {
                    let taken = rest(&stmts[i + 1..], arm, whole);
                    (stmts[i + 1..i + 1 + taken].to_vec(), taken)
                }
                None => (Vec::new(), 0),
            };
            cases.push((values, own, false, arm.span));
            cases.push((
                untested(last, discriminant)?,
                then.clone(),
                last.constants.is_empty(),
                last.span,
            ));
            let taken = i + 1 + taken;
            return (!whole || taken == stmts.len()).then_some((cases, taken));
        }
        let (tested, body) = guarded(test, then, arm.guard)?;
        cases.push((
            values_of(&tested, &arm.constants, false, discriminant)?,
            body,
            false,
            arm.span,
        ));
        i += 1;
        match otherwise {
            // The rest, the `else`'s, all of it.
            Some(otherwise) => {
                let (rest, taken) = chain(otherwise, &left, true, discriminant)?;
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

/// An arm's `if`, its test and its statements, as its case has them: of a
/// guarded arm whose guard the test joins, `n === C && on`, its pattern's
/// test, and what it does in an `if` of the guard, `if (on) { .. }`; else
/// as they are, the guard's `if` in its statements (ADR 0629).
fn guarded(test: &Expr, then: &[Stmt], guard: Option<js::Span>) -> Option<(Expr, Vec<Stmt>)> {
    let mut tests = Vec::new();
    alternatives(test, Op::And, &mut tests);
    let at = guard
        .and_then(|guard| (tests.iter()).position(|t| test_at(t, js::Span::NONE).is_some_and(|at| within(at, guard))));
    let Some(at) = at else {
        return Some((test.clone(), then.to_vec()));
    };
    let join = |tests: &[&Expr]| {
        (tests.iter())
            .map(|&t| t.clone())
            .reduce(|a, b| Expr::bin(Op::And, a, b))
    };
    let (pattern, guard) = (join(&tests[..at])?, join(&tests[at..])?);
    let span = guard.span;
    Some((pattern, vec![StmtKind::If(guard, then.to_vec(), None).at(span)]))
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
fn untested(arm: &SwitchArm, discriminant: &Option<Expr>) -> Option<Values> {
    let values = |values| Values {
        values,
        ..Values::default()
    };
    match arm.constants.as_slice() {
        [] => Some(values(Vec::new())),
        [Constant::Variant(value)] => Some(values(vec![value.clone()])),
        // Variants of an untagged enum, of what the `switch` tells, `typeof
        // value` (ADR 0668).
        kinds => {
            let Some(Expr {
                kind: ExprKind::Unary(UnaryOp::Typeof, subject),
                ..
            }) = discriminant
            else {
                return None;
            };
            let mut untested = Values::default();
            for constant in kinds {
                match constant {
                    Constant::Kind(TypeofCase::Typeof(case, excluding)) => {
                        untested.values.push(Expr::str(*case));
                        (untested.excluding).extend(excluding.iter().map(|e| of_subject(e, subject)));
                    }
                    Constant::Kind(TypeofCase::Tested(test)) => untested.tested.push(of_subject(test, subject)),
                    Constant::Kind(TypeofCase::Inside(case, test)) => {
                        untested.inside.push((case, of_subject(test, subject)));
                    }
                    _ => return None,
                }
            }
            Some(untested)
        }
    }
}

/// `test`, of `SUBJECT`, of `subject`.
fn of_subject(test: &Expr, subject: &Expr) -> Expr {
    let mut test = test.clone();
    test.each_mut(&mut |e| {
        if matches!(&e.kind, ExprKind::Var(v) if v == SUBJECT) {
            *e = subject.clone();
        }
    });
    test
}

/// The values of `tests`, each a test of a variant of an untagged enum,
/// `constants`, in any order, as MIR tests an or-pattern's in its
/// variants': `typeof value === "string"`'s `"string"`, and of what
/// `typeof` doesn't tell, its test (ADR 0668); the values in the order
/// the pattern names them. What `discriminant` is, `typeof value`, or the
/// first says.
fn kinds_of(tests: &[&Expr], constants: &[Constant], discriminant: &mut Option<Expr>) -> Option<Values> {
    let mut left: Vec<(usize, &TypeofCase)> = (constants.iter().enumerate())
        .map(|(k, c)| match c {
            Constant::Kind(case) => Some((k, case)),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let mut kinds = Values::default();
    let mut values = Vec::new();
    for &test in tests {
        let mut ands = Vec::new();
        alternatives(test, Op::And, &mut ands);
        // What it tests: of `typeof value === "string"`, `value`, and of
        // `value === undefined`, which is `typeof value === "undefined"`
        // (ADR 0676), too; of `Array.isArray(value)`, what's in it the
        // variant's test is of.
        let typeof_of = |e: &Expr| match &e.kind {
            ExprKind::Binary(Op::Eq, a, b) => match (&a.kind, &b.kind) {
                (ExprKind::Unary(UnaryOp::Typeof, subject), ExprKind::Str(case)) => {
                    Some((subject.clone(), case.clone()))
                }
                (_, ExprKind::Undefined) => Some((a.clone(), "undefined".to_string())),
                _ => None,
            },
            _ => None,
        };
        let at = (left.iter()).position(|(_, case)| match case {
            TypeofCase::Typeof(case, excluding) => typeof_of(ands[0]).is_some_and(|(subject, of)| {
                of == *case
                    && ands.len() == excluding.len() + 1
                    && (ands[1..].iter().zip(excluding))
                        .all(|(and, e)| alike(and, &Expr::unary(UnaryOp::Not, of_subject(e, &subject))))
                    && compared(discriminant, &Expr::unary(UnaryOp::Typeof, (*subject).clone()))
            }),
            TypeofCase::Tested(of) | TypeofCase::Inside(_, of) => {
                let mut within = Vec::new();
                test.clone().each_mut(&mut |e| within.push(e.clone()));
                let subject = match &*discriminant {
                    Some(Expr {
                        kind: ExprKind::Unary(UnaryOp::Typeof, subject),
                        ..
                    }) => Some((**subject).clone()),
                    Some(_) => None,
                    None => within.into_iter().find(|s| alike(&of_subject(of, s), test)),
                };
                subject.is_some_and(|subject| {
                    alike(&of_subject(of, &subject), test)
                        && compared(discriminant, &Expr::unary(UnaryOp::Typeof, subject))
                })
            }
        })?;
        match left.remove(at) {
            (k, TypeofCase::Typeof(case, excluding)) => {
                let Some(Expr {
                    kind: ExprKind::Unary(_, subject),
                    ..
                }) = &*discriminant
                else {
                    unreachable!("a `typeof` it tests")
                };
                let ExprKind::Binary(_, _, value) = &ands[0].kind else {
                    unreachable!("a `typeof` test")
                };
                // The case's word, where the test's written: `undefined`'s
                // `"undefined"`.
                values.push((k, Expr::str(*case).or_at(value.span)));
                kinds.excluding.extend(excluding.iter().map(|e| of_subject(e, subject)));
            }
            (_, TypeofCase::Tested(_)) => kinds.tested.push(test.clone()),
            (_, TypeofCase::Inside(case, _)) => kinds.inside.push((case, test.clone())),
        }
    }
    values.sort_by_key(|&(k, _)| k);
    kinds.values = values.into_iter().map(|(_, value)| value).collect();
    Some(kinds)
}

/// The values `test` compares what it tests to, of an arm of `constants`,
/// each in turn: `n === 1 || n === 2`'s, or, `negated`, `n !== 1 && n !==
/// 2`'s, of what `discriminant` is, or the first says, `n`.
fn values_of(test: &Expr, constants: &[Constant], negated: bool, discriminant: &mut Option<Expr>) -> Option<Values> {
    let (joined, compares) = match negated {
        false => (Op::Or, Op::Eq),
        true => (Op::And, Op::Ne),
    };
    let mut tests = Vec::new();
    alternatives(test, joined, &mut tests);
    if tests.len() != constants.len() {
        return None;
    }
    // Variants of an untagged enum, which MIR tests as they are, each by
    // its kind, never negated (ADR 0668).
    if constants.iter().any(|c| matches!(c, Constant::Kind(_))) {
        return match negated {
            true => None,
            false => kinds_of(&tests, constants, discriminant),
        };
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
            Constant::Kind(_) => (false, js::Span::NONE),
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
    Some(Values {
        values,
        ..Values::default()
    })
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

/// Where `test`, of an `if` at `span`, is written: of tests `&&` joins
/// where the program didn't, a pattern's and its guard's, the first's.
fn test_at(test: &Expr, span: js::Span) -> Option<u32> {
    match &test.kind {
        ExprKind::Binary(Op::And, first, _) if test.span.is_none() => test_at(first, span),
        _ => [test.span, span].into_iter().find(|s| !s.is_none()).map(|s| s.lo),
    }
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

/// A block's `break` in a `switch` its statements end with, `l: { if (c) {
/// .. } else { switch (k) { case 1: .. break l; } } }`, which goes where
/// the `switch`'s own `break` does, nothing between: that `break`, as a
/// person writes a case's end (ADR 0731); and the block's label, which
/// nothing names now, gone.
pub(super) fn breaks_past_switches(stmts: &mut Vec<Stmt>) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            let label = label.clone();
            ending_switches(body, &mut |case| case_breaks(case, &label));
        }
    }
    unlabeled(stmts);
}

/// Each `switch` `stmts` end with, through the branches of an `if` they
/// end with and a block's statements, each of its cases given to `f`.
fn ending_switches(stmts: &mut [Stmt], f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::If(_, a, b)) => {
            ending_switches(a, f);
            if let Some(b) = b {
                ending_switches(b, f);
            }
        }
        Some(StmtKind::Labeled(_, body)) => ending_switches(body, f),
        Some(StmtKind::Switch { cases, default, .. }) => {
            for case in js::case_lists_mut(cases, default) {
                f(case);
            }
        }
        _ => {}
    }
}

/// Each `break label` of a case's `stmts`, outside a loop or `switch` of
/// its own, which a bare `break` would leave instead: a bare `break`.
fn case_breaks(stmts: &mut [Stmt], label: &str) {
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::Break(Some(l)) if l == label => stmt.kind = StmtKind::Break(None),
            StmtKind::If(_, a, b) => {
                case_breaks(a, label);
                if let Some(b) = b {
                    case_breaks(b, label);
                }
            }
            StmtKind::Labeled(_, body) => case_breaks(body, label),
            _ => {}
        }
    }
}
