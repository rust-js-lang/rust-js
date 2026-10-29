//! A `match` of constants the program wrote, its arms' chain of tests
//! settled: a `switch`, as JS writes one, `switch (tag) { case
//! FunctionComponent: .. }` (ADR 0629). Which `match` is one is
//! `hints::written_switches`'; how its arms' tests read, its chain's.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp};
use crate::lower::mir::hints::{Constant, SUBJECT, Switch, SwitchArm};
use crate::lower::untagged::TypeofCase;

use super::bindings::declared;
use super::loop_labels::{LOOP_LABEL, continued, held_subject, labeled_breaks, labels_in};
use super::{first_written, is_temporary, relabeled, unlabeled, written_loop};

/// Each chain of `body` that tests the arms of a `match` of `switches`, in
/// the order written, a `switch`; a `break` in it of a loop around it, of
/// that loop's label, the program's of `loops` where it wrote one.
pub(super) fn switches(body: &mut Vec<Stmt>, switches: &[Switch], loops: &[(js::Span, bool, Option<String>)]) {
    if switches.is_empty() {
        return;
    }
    walk(body, switches, loops, &mut Vec::new(), None, false);
}

/// The loop a `switch` is in: the label a `break` of it names, whether
/// the program wrote it, and whether one does.
struct Around<'l> {
    label: &'l str,
    written: bool,
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
    turn: bool,
) {
    let mut i = 0;
    while i < list.len() {
        written_after(list, i, switches);
        if let Some((switch, taken)) = switch_at(list, i, switches, around.as_deref_mut(), turn) {
            list.splice(i..i + taken, switch);
            if discriminant_in_place(list, i) {
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
                walk(a, switches, loops, labels, around.as_deref_mut(), false);
                if let Some(b) = b {
                    walk(b, switches, loops, labels, around.as_deref_mut(), false);
                }
            }
            StmtKind::Labeled(_, body) => walk(body, switches, loops, labels, around.as_deref_mut(), false),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                walk(a, switches, loops, labels, around.as_deref_mut(), false);
                walk(b, switches, loops, labels, around.as_deref_mut(), false);
            }
            StmtKind::Switch { cases, default, .. } => {
                for body in js::case_lists_mut(cases, default) {
                    walk(body, switches, loops, labels, around.as_deref_mut(), false);
                }
            }
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => {
                // Its label, a readable one where MIR made it, `bb5`: the
                // program's, or `loop`, apart from the labels around it and
                // in it.
                let written = written_loop(loops, span).and_then(|(.., l)| l.clone());
                let (readable, written) = match label.as_deref() {
                    Some(l) if !crate::lower::mir::is_block_label(l) => (l.to_string(), true),
                    _ => {
                        let mut taken: HashSet<String> = labels.iter().cloned().collect();
                        labels_in(body, &mut taken);
                        let readable = (written.clone().into_iter())
                            .chain((0..).map(|n| match n {
                                0 => LOOP_LABEL.to_string(),
                                n => format!("{LOOP_LABEL}${n}"),
                            }))
                            .find(|name| !taken.contains(name))
                            .expect("a name no label has");
                        (readable.clone(), written.as_ref() == Some(&readable))
                    }
                };
                let mut inner = Around {
                    label: &readable,
                    written,
                    named: false,
                };
                labels.push(readable.clone());
                walk(body, switches, loops, labels, Some(&mut inner), true);
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

/// `if (t) { exit } else { own; rest }`, an arm's test whose branch leaves,
/// `rest` written after the `match`: `if (t) { exit } else { own } rest`,
/// `case 'inert': break; default: return true; } console.error(..);` (rd49,
/// ADR 0629, amended): `exit` leaves, so `rest` follows `own` either way.
fn written_after(list: &mut Vec<Stmt>, i: usize, switches: &[Switch]) {
    let of = |at| (switches.iter()).find(|s: &&Switch| (s.arms.iter()).any(|arm| within(at, arm.pattern)));
    let Some(written) = tested_at(&list[i]).and_then(of) else {
        return;
    };
    let StmtKind::If(_, exit, Some(otherwise)) = &mut list[i].kind else {
        return;
    };
    let past = |s: &Stmt| written_at(s).is_some_and(|at| at >= written.span.hi);
    let Some(k) = otherwise.iter().position(past) else {
        return;
    };
    if !exit.last().is_some_and(js::ends) || !otherwise[k..].iter().all(|s| written_at(s).is_none() || past(s)) {
        return;
    }
    let rest = otherwise.split_off(k);
    if let StmtKind::If(_, _, otherwise) = &mut list[i].kind
        && otherwise.as_ref().is_some_and(Vec::is_empty)
    {
        *otherwise = None;
    }
    list.splice(i + 1..i + 1, rest);
}

/// A temporary only the test of the `switch` at `list[i]` reads, `const _5
/// = s.charCodeAt(i);` just before, but for what's declared, unset, `let
/// v;`: its value, read in place, `switch (s.charCodeAt(i))`; not one
/// whose `if` chain will read it again (`loop_labels::held_subject`).
/// Whether it was, the `switch` then at `list[i - 1]`.
fn discriminant_in_place(list: &mut Vec<Stmt>, i: usize) -> bool {
    if held_subject(&list[i]) {
        return false;
    }
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
        return true;
    }
    false
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
    turn: bool,
) -> Option<(Vec<Stmt>, usize)> {
    let first = &stmts[at];
    let at_test = tested_at(first)?;
    let written = (switches.iter()).find(|s| (s.arms.iter()).any(|arm| within(at_test, arm.pattern)))?;
    let arms: Vec<&SwitchArm> = written.arms.iter().collect();
    let left: Vec<(&SwitchArm, usize)> = arms.iter().map(|&arm| (arm, 0)).collect();
    let mut discriminant = None;
    let (mut cases, mut taken) = chain(&stmts[at..], &left, false, &mut discriminant)?;
    let discriminant = discriminant?;
    // Its cases as its arms are written, whatever order MIR tests them in;
    // an arm's alternatives MIR tested apart, a guard's each, one case.
    cases.sort_by_key(|case| case.3.lo);
    let mut cases = stacked(cases)?;
    // What follows it to its list's end, where one case alone goes on to
    // it and it's that arm's, written there or MIR's: that case's, `'.' =>
    // break` of a loop's `break` after it.
    let after = &stmts[at + taken..];
    let going: Vec<usize> = (0..cases.len())
        .filter(|&c| !cases[c].1.last().is_some_and(js::ends))
        .collect();
    let falling = arms.iter().any(|arm| arm.falls);
    // Of those, each that goes on to what's written in an arm MIR placed
    // after the chain: not a guarded arm's that falls to nothing (ADR
    // 0636), whose failed guard goes past the `match`, as `_ => {}` does,
    // the `_`'s jump there in its chain's list after it.
    let mut going_after: Vec<usize> = (going.iter().copied())
        .filter(|&c| (arms.iter()).any(|arm| arm.span == cases[c].3 && (arm.guard.is_none() || arm.falls)))
        .collect();
    // Each arm a case: one that does nothing, left out, goes on too. Of
    // the `default`, a `_`'s, what follows that's written in it or MIR's,
    // up to the first of what's written past it, which stays after the
    // `switch`, where nothing there reads what it declares: what MIR placed
    // after the chain of the arms before a last `_ => { .. }`, which only
    // that case goes on to (ADR 0629, amended). One that does nothing, `_
    // => {}`, is left out where it's then nothing (`breaks_past_switches`).
    let mut absorbed = 0;
    if let [only] = going[..]
        && !falling
        && cases.len() == arms.len()
        && !after.is_empty()
    {
        let arm = cases[only].3;
        let own = |s: &Stmt| written_at(s).is_none_or(|at| within(at, arm));
        let n = match cases[only].2 {
            false if after.iter().all(own) => after.len(),
            false => 0,
            true => {
                let n = (after.iter()).take_while(|s| own(s)).count();
                let rest = &after[n..];
                match declared(&after[..n]).all(|name| js::mentions_in(rest, name) == 0) {
                    true => n,
                    false => 0,
                }
            }
        };
        cases[only].1.extend(after[..n].iter().cloned());
        taken += n;
    }
    // Of a `match` whose guarded arms fall to the next (ADR 0905), what
    // follows that's written in the arm they fall to last, up to the first
    // of what's written past it, where nothing there reads what it
    // declares: each case's that goes on to it, which each case that falls
    // then has of the next's, and only the last of them its own. Of that
    // arm's own case too where it's guarded and falls to nothing, `B | C if
    // on => { .. }` before `_ => {}`: what follows is its statements, whose
    // guard's `if` goes on to them (ADR 0905, amended).
    if falling
        && cases.len() == arms.len()
        && let Some(arm) = (after.first())
            .and_then(written_at)
            .and_then(|at| arms.iter().find(|arm| within(at, arm.span)))
    {
        let own = |s: &Stmt| written_at(s).is_none_or(|at| within(at, arm.span));
        let n = (after.iter()).take_while(|s| own(s)).count();
        if declared(&after[..n]).all(|name| js::mentions_in(&after[n..], name) == 0) {
            let own_case: Vec<usize> = (going.iter().copied())
                .filter(|&c| cases[c].3 == arm.span && !going_after.contains(&c))
                .collect();
            going_after.extend(own_case);
            for &c in &going_after {
                cases[c].1.extend(after[..n].iter().cloned());
            }
            absorbed = n;
            taken += n;
        }
    }
    // Of guarded arms that fall through, a `break` after them, the list's
    // last, which each case that goes on goes on to, MIR's shared way out
    // of the loop or of a block: each such case's, as a jump after an
    // `else`'s chain is (`chain`), so the `switch` ends where its cases do.
    if falling
        && let [
            jump @ Stmt {
                kind: StmtKind::Break(_),
                ..
            },
        ] = &stmts[at + taken..]
    {
        for (_, body, ..) in cases.iter_mut().filter(|case| !case.1.last().is_some_and(js::ends)) {
            body.push(jump.clone());
        }
        taken += 1;
    }
    // Each arm of `null`, which `typeof` says is an object, an `if` of its
    // test in `case "object"`; and each of an array, an `if` of its test
    // before the `switch`, and so what the others' leave out.
    let tested = cases.len();
    nulls_inside(&mut cases)?;
    let before = arrays_before(&mut cases)?;
    // A `break` of no label in a case, of the loop around it, which in a
    // `switch` would leave the `switch`: of the loop's label, where the
    // program wrote one; else, where the loop's turn ends at the `switch`,
    // the `switch`'s, a `break;` after it leaving the loop, each case that
    // goes on to the next turn its `continue;` (`framed`); and otherwise no
    // `switch`, its arms the `if` chain they are, as no label the program
    // didn't write is JS's (#389, the user's decision). But of guarded arms
    // that fall through (ADR 0905), whose `if` chain MIR's shared tests make
    // only of labeled blocks, `loop` still, written up for the user.
    let mut framed = false;
    if cases.iter().any(|(_, body, ..)| js::breaks(body, None)) {
        let around = around?;
        if !around.written && turn && stmts.len() == at + taken && before.is_empty() {
            framed = true;
        } else if around.written || falling {
            for (_, body, ..) in cases.iter_mut() {
                labeled_breaks(body, around.label);
            }
            around.named = true;
        } else {
            return None;
        }
    }
    // Each guarded arm's case that falls to the next, its own statements
    // the guard's `if`; what follows, which went on to each, the last's
    // alone (ADR 0905).
    let mut falls = vec![false; cases.len()];
    if falling {
        if cases.len() != tested {
            return None;
        }
        falls = fallen(&mut cases, &arms)?;
        let holding = going_after.iter().filter(|&&c| !falls[c]).count();
        if absorbed > 0 && holding > 1 {
            return None;
        }
    }
    // A last `_ => {}`'s `default`, MIR's copy of what's written past the
    // `match`, ending in its `return`, where that's the chain's end: after
    // the `switch`, as written, `switch (..) { .. } return bailout(fiber);`,
    // each case that went on to it going on past the `switch` (ADR 0905,
    // amended).
    let mut past = Vec::new();
    if !framed
        && before.is_empty()
        && stmts.len() == at + taken
        && let Some((_, body, true, _)) = cases.last()
        && body.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(_)))
        && (body.iter()).all(|s| written_at(s).is_some_and(|at| !within(at, written.span)))
        && declared(body).next().is_none()
    {
        past = cases.pop().map(|(_, body, ..)| body).unwrap_or_default();
        falls.pop();
    }
    // A `return` MIR copied into a case of what follows the `match`, the
    // `return` after the `switch`: the case's `break`, as written.
    if let Some(Stmt {
        kind: StmtKind::Return(value),
        ..
    }) = past.last().or(stmts.get(at + taken))
    {
        for (_, body, ..) in cases.iter_mut() {
            returns_broken(body, value.as_ref(), written.span);
        }
    }
    let mut switched = Vec::new();
    let mut default = None;
    for ((values, body, last, _), falls) in cases.into_iter().zip(falls) {
        match last {
            true => default = Some(body),
            false => switched.push(js::Case {
                values: values.values,
                body,
                falls,
            }),
        }
    }
    // Where the `match` is written, not its first arm's pattern, which the
    // first case's constant is (ADR 0630).
    let span = written.span;
    let mut switch = StmtKind::Switch {
        discriminant,
        cases: switched,
        default,
    };
    if framed {
        continued(&mut switch);
    }
    // Each `if` before it, its `else` the rest, where it goes on.
    let left = framed.then(|| StmtKind::Break(None).at(js::Span::NONE));
    let switched = std::iter::once(switch.at(span)).chain(past).chain(left).collect();
    let written = (before.into_iter().rev()).fold(switched, |rest, (test, body, at)| {
        match body.last().is_some_and(js::ends) {
            true => std::iter::once(StmtKind::If(test, body, None).at(at))
                .chain(rest)
                .collect(),
            false => vec![StmtKind::If(test, body, Some(rest)).at(at)],
        }
    });
    Some((written, taken))
}

/// `cases`, in the order their arms are written, each arm's one: of an
/// arm whose constants MIR tested apart, a guarded or-pattern's, each
/// with the guard (ADR 0905), its values stacked in the order tested,
/// where the statements of each are alike, as they are of one arm.
fn stacked(cases: Vec<Case>) -> Option<Vec<Case>> {
    let mut stacked: Vec<Case> = Vec::new();
    for case in cases {
        match stacked.last_mut() {
            Some(last) if last.3 == case.3 => {
                let (values, body, ..) = case;
                let plain = |v: &Values| v.tested.is_empty() && v.excluding.is_empty() && v.inside.is_empty();
                if !plain(&last.0) || !plain(&values) || !js::stmts_alike(&last.1, &body) {
                    return None;
                }
                last.0.values.extend(values.values);
            }
            _ => stacked.push(case),
        }
    }
    Some(stacked)
}

/// Of `cases`, each in the order its arm is written, each of an arm that
/// falls to the next (`SwitchArm::falls`), whose statements are the
/// guard's `if` and then the next case's: the guard's `if` alone, its case
/// falling through to the next (ADR 0905). Whether each falls, or `None`
/// where one's statements aren't so. Where its guard fails, a case only
/// tested, then goes on to the next's statements, as Rust goes on to the
/// next arm; and where it holds, it leaves, as its arm's statements did,
/// a `break;` of the `switch` where they go on past the `match`.
fn fallen(cases: &mut [Case], arms: &[&SwitchArm]) -> Option<Vec<bool>> {
    let mut falls = vec![false; cases.len()];
    let mut own = Vec::new();
    for (c, case) in cases.iter().enumerate() {
        let k = arms.iter().position(|arm| arm.span == case.3)?;
        if !arms[k].falls {
            continue;
        }
        // The next case is the next arm's, which Rust tries next.
        let next = cases
            .get(c + 1)
            .filter(|next| arms.get(k + 1).is_some_and(|arm| arm.span == next.3))?;
        // Of the `default` next, a last `_`'s, the guard's `if` alone,
        // which leaves where it holds, is one the chain goes on from to the
        // `_`'s statements, as the case falls to them (ADR 0636, amended).
        let alone = match (&case.1[..], next.2) {
            (
                [
                    stmt @ Stmt {
                        kind: StmtKind::If(_, then, None),
                        ..
                    },
                ],
                true,
            ) if then.last().is_some_and(js::ends) => Some(stmt.clone()),
            _ => guard_alone(&case.1, &next.1),
        };
        own.push((c, alone?));
        falls[c] = true;
    }
    for (c, stmt) in own {
        cases[c].1 = vec![stmt];
    }
    Some(falls)
}

/// Of a case's statements, `body`, those of the guard's `if` followed by
/// those of the next case, `next`: the guard's `if`, which leaves where the
/// guard holds, and else goes on, having only tested. MIR's are one of
/// `if (guard) { .. } next`, its `if` leaving; `if (guard) { .. } else {
/// next }`; `if (!guard) { next } ..`, what follows leaving, as MIR tests a
/// guard whose arm does nothing; or `if (!guard) { next } else { .. }`.
/// Of `&&`, tested in parts, `if (a) { if (b) { .. } }`, one test, `if (a
/// && b)`, React's finishConcurrentRender's (ADR 0905, amended).
fn guard_alone(body: &[Stmt], next: &[Stmt]) -> Option<Stmt> {
    let (first, rest) = body.split_first()?;
    let StmtKind::If(test, then, otherwise) = &first.kind else {
        return None;
    };
    let (mut test, mut then) = (test.clone(), then);
    while otherwise.is_none()
        && let [part] = &then[..]
        && let StmtKind::If(t, inner, None) = &part.kind
    {
        (test, then) = (Expr::bin(Op::And, test, t.clone()), inner);
    }
    // Each way it goes on to the next only tests.
    let mut assigns = false;
    test.clone()
        .each_mut(&mut |e| assigns |= matches!(e.kind, ExprKind::Assigned(..)));
    if assigns {
        return None;
    }
    // What leaves where the guard holds: of statements that go on past the
    // `match`, a `break;` of the `switch` after them.
    let leaving = |stmts: &[Stmt]| {
        let mut stmts = stmts.to_vec();
        if !stmts.last().is_some_and(js::ends) {
            stmts.push(StmtKind::Break(None).at(js::Span::NONE));
        }
        stmts
    };
    let alike = js::stmts_alike;
    let held = match otherwise {
        None if alike(rest, next) && then.last().is_some_and(js::ends) => (test.clone(), then.clone()),
        None if alike(then, next) && rest.last().is_none_or(js::ends) => (test.clone().negated_test(), leaving(rest)),
        Some(otherwise) if rest.is_empty() && alike(otherwise, next) => (test.clone(), leaving(then)),
        Some(otherwise) if rest.is_empty() && alike(then, next) => (test.clone().negated_test(), leaving(otherwise)),
        // `if (guard) { .. } else { next } rest`, its `if` leaving: the
        // `else` and what follows are the next's, `else if` of three arms
        // falling, React's getTag (ADR 0905, amended).
        Some(otherwise) if then.last().is_some_and(js::ends) && alike(&[otherwise.as_slice(), rest].concat(), next) => {
            (test.clone(), then.clone())
        }
        // And `if (!guard) { next } else { .. } rest`, its `else` leaving (#389c).
        Some(otherwise) if otherwise.last().is_some_and(js::ends) && alike(&[then, rest].concat(), next) => {
            (test.clone().negated_test(), otherwise.clone())
        }
        _ => return None,
    };
    let (test, then) = held;
    Some(StmtKind::If(test, then, None).at(first.span))
}

/// A case's `if (!guard) { break; } ..`, MIR's test of a guard whose
/// failing goes past the `match`, where what follows leaves: `if (guard) {
/// .. }`, as the arm is written, its case going on past the `switch` where
/// the guard fails (ADR 0905, amended); of `a || b`, tested in parts, `if
/// (!a) { if (!b) { break; } }`, `if (a || b) { .. }`. Only of a guard
/// written of the arm what follows is written in.
fn guard_held(stmts: &mut Vec<Stmt>, switches: &[Switch]) {
    let [first, rest @ ..] = &stmts[..] else {
        return;
    };
    // Of a guard MIR tests in parts, `if (!a) { if (!b) { break; } }`, of
    // `a || b`: each part.
    let mut tests = Vec::new();
    let mut then = std::slice::from_ref(first);
    while let [
        Stmt {
            kind: StmtKind::If(test, inner, None),
            ..
        },
    ] = then
    {
        tests.push(test.clone());
        then = inner;
    }
    let Some(test) = tests.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)) else {
        return;
    };
    let broken = matches!(
        then,
        [Stmt {
            kind: StmtKind::Break(None),
            ..
        }]
    );
    if !broken || !rest.last().is_some_and(js::ends) {
        return;
    }
    // A guard written of the arm what follows is written in: not `I if
    // legacy_off => {}` before `I => { .. }`, whose `if (legacyOff) {
    // break; }` is as written (ADR 0905, amended).
    let arm = (switches.iter().flat_map(|s| &s.arms))
        .find(|arm| (arm.guard).is_some_and(|guard| test_at(&test, first.span).is_some_and(|at| within(at, guard))));
    if !arm.is_some_and(|arm| (rest.iter()).all(|s| written_at(s).is_some_and(|at| within(at, arm.span)))) {
        return;
    }
    let held = StmtKind::If(test.negated_test(), rest.to_vec(), None).at(first.span);
    *stmts = vec![held];
}

/// Each `return value;` of a case's `stmts`, outside a loop, a `switch` or
/// a `try` of their own, that's written past the `match` at `written`,
/// MIR's copy of the `return` the `switch` is followed by: the case's
/// `break;`, which goes on to that `return`. One that ends the case,
/// which ends there anyway, gone.
pub(super) fn returns_broken(stmts: &mut Vec<Stmt>, value: Option<&Expr>, written: js::Span) {
    fn each(stmts: &mut [Stmt], value: Option<&Expr>, written: js::Span) {
        for stmt in stmts.iter_mut() {
            match &mut stmt.kind {
                StmtKind::Return(returned)
                    if match (&*returned, value) {
                        (None, None) => true,
                        (Some(returned), Some(value)) => alike(returned, value) && !within(returned.span.lo, written),
                        _ => false,
                    } && !within(stmt.span.lo, written) =>
                {
                    stmt.kind = StmtKind::Break(None);
                }
                StmtKind::If(_, a, b) => {
                    each(a, value, written);
                    if let Some(b) = b {
                        each(b, value, written);
                    }
                }
                StmtKind::Labeled(_, body) => each(body, value, written),
                _ => {}
            }
        }
    }
    each(stmts, value, written);
    if matches!(stmts.last().map(|s| &s.kind), Some(StmtKind::Break(None))) {
        stmts.pop();
    }
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

/// The cases `stmts` test of the arms `left`, in order, and how many
/// statements they are: an `if` of each arm's test, its `else` the rest's,
/// or, where its branch leaves, the next; what's left the last arm's, all
/// of `stmts` where they're all the `match`'s, `whole`, as an `else` is,
/// else what's written in it. And of an arm, an `if` of its test negated,
/// the branch of the arms after it, `if (c !== ".") { .. }`, as a `match`
/// whose arm does nothing, or leaves, MIR tests. An arm's constants may be
/// tested apart, each in turn, as MIR tests a guarded or-pattern's
/// alternatives, the guard in each (ADR 0905): a case of each; `left` has
/// how many of each arm's are tested. `discriminant`, what each test
/// compares, as the first tells.
fn chain(
    stmts: &[Stmt],
    left: &[(&SwitchArm, usize)],
    whole: bool,
    discriminant: &mut Option<Expr>,
) -> Option<(Vec<Case>, usize)> {
    let mut cases = Vec::new();
    let mut i = 0;
    let mut left = left.to_vec();
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
                .position(|(arm, _)| {
                    !arm.constants.is_empty() && tested_at(stmt).is_some_and(|at| within(at, arm.pattern))
                })
                .map(|k| (k, test, then, otherwise)),
            _ => None,
        };
        let Some((k, test, then, otherwise)) = tested else {
            // The rest, untested: the last arm's, the one left. A chain
            // begun past an arm's test leaves that arm and the last, a `_`
            // or one Rust proved, which no test is of: no `switch`.
            let [(arm, 0)] = left[..] else {
                return None;
            };
            let taken = rest(&stmts[i..], arm, whole);
            let values = untested(arm, discriminant)?;
            cases.push((values, stmts[i..i + taken].to_vec(), arm.constants.is_empty(), arm.span));
            return Some((cases, i + taken));
        };
        let (arm, from) = left[k];
        // Its test negated, the branch of the arms after it: its own the
        // `else`, or what's after where that leaves.
        let mut negated = discriminant.clone();
        if let Some(values) = values_of(test, &arm.constants[from..], true, &mut negated) {
            left.remove(k);
            if left.is_empty() || left.iter().any(|&(_, from)| from > 0) {
                return None;
            }
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
            match left[..] {
                // The last arm's, untested.
                [(last, _)] => cases.push((
                    untested(last, discriminant)?,
                    then.clone(),
                    last.constants.is_empty(),
                    last.span,
                )),
                // The arms after it, all of the branch.
                _ => {
                    let (rest, taken) = chain(then, &left, true, discriminant)?;
                    if taken != then.len() {
                        return None;
                    }
                    cases.extend(rest);
                }
            }
            let taken = i + 1 + taken;
            return (!whole || taken == stmts.len()).then_some((cases, taken));
        }
        let (tested, body, held) = guarded(test, then, arm.guard)?;
        // The constants it tests: the next of its arm's.
        let mut each = Vec::new();
        alternatives(&tested, Op::Or, &mut each);
        let to = from + each.len();
        let constants = arm.constants.get(from..to)?;
        cases.push((
            values_of(&tested, constants, false, discriminant)?,
            body,
            false,
            arm.span,
        ));
        match to == arm.constants.len() {
            true => {
                left.remove(k);
            }
            false => left[k].1 = to,
        }
        i += 1;
        match otherwise {
            // The rest, the `else`'s, all of it; or but a jump after them,
            // which each of its cases that goes on goes on to, where one is
            // each value's, the last arm's there: each such case's.
            Some(otherwise) => {
                let (mut rest, taken) = chain(otherwise, &left, true, discriminant)?;
                if taken != otherwise.len() {
                    let ([jump], Some(&(last, _))) = (&otherwise[taken..], left.last()) else {
                        return None;
                    };
                    if !a_jump(jump) || !rest.iter().any(|case| case.3 == last.span) {
                        return None;
                    }
                    for case in rest.iter_mut().filter(|case| !case.1.last().is_some_and(js::ends)) {
                        case.1.push(jump.clone());
                    }
                }
                cases.extend(rest);
                return Some((cases, i));
            }
            // The next, where this one leaves.
            None if held.last().is_some_and(js::ends) => {}
            // The arms after it do nothing.
            None => return Some((cases, i)),
        }
    }
    // Each arm tested: what's after them isn't the `match`'s, but in an
    // `else`, which nothing reaches.
    (!whole || i == stmts.len()).then_some((cases, i))
}

/// Whether `stmt` only jumps: a `break`, a `continue`, or a `return` of
/// nothing, a name or a literal, which reads the same where it's copied.
fn a_jump(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Return(None) => true,
        StmtKind::Return(Some(e)) => {
            matches!(e.kind, ExprKind::Var(_) | ExprKind::Null | ExprKind::Undefined) || literal(e)
        }
        _ => false,
    }
}

/// An arm's `if`, its test and its statements, as its case has them: of a
/// guarded arm whose guard the test joins, `n === C && on`, its pattern's
/// test, and what it does in an `if` of the guard, `if (on) { .. }`; else
/// as they are, the guard's `if` in its statements (ADR 0629). A guard MIR
/// tests in part in an `if` alone inside, `if (n === C && a) { if (b) { ..
/// } }`, is one `if` of it all, `if (a && b) { .. }`: neither has an
/// `else`, so each way on is the same. And what the arm does once its
/// guard holds, whose end says whether it leaves.
fn guarded(test: &Expr, then: &[Stmt], guard: Option<js::Span>) -> Option<(Expr, Vec<Stmt>, Vec<Stmt>)> {
    // A test of the guard's, wherever MIR tests it.
    let of_guard = |t: &Expr| guard.is_some_and(|guard| test_at(t, js::Span::NONE).is_some_and(|at| within(at, guard)));
    // `then`'s `if`s alone of the guard: their tests, and what they hold.
    let peeled = |mut then: Vec<Stmt>| {
        let mut inner = Vec::new();
        while let [
            Stmt {
                kind: StmtKind::If(test, held, None),
                ..
            },
        ] = &then[..]
            && of_guard(test)
        {
            inner.push(test.clone());
            then = held.clone();
        }
        (inner, then)
    };
    let mut tests = Vec::new();
    alternatives(test, Op::And, &mut tests);
    let Some(at) = tests.iter().position(|t| of_guard(t)) else {
        let (_, held) = peeled(then.to_vec());
        return Some((test.clone(), then.to_vec(), held));
    };
    let join = |tests: Vec<Expr>| tests.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b));
    let pattern = join(tests[..at].iter().map(|&t| t.clone()).collect())?;
    let own = join(tests[at..].iter().map(|&t| t.clone()).collect())?;
    let span = own.span;
    let (inner, held) = peeled(then.to_vec());
    let joined = join(std::iter::once(own).chain(inner).collect())?;
    Some((pattern, vec![StmtKind::If(joined, held.clone(), None).at(span)], held))
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
pub(super) fn alike(a: &Expr, b: &Expr) -> bool {
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
        ExprKind::Binary(Op::And | Op::Or, first, _) if test.span.is_none() => test_at(first, span),
        ExprKind::Unary(UnaryOp::Not, inner) if test.span.is_none() => test_at(inner, span),
        _ => [test.span, span].into_iter().find(|s| !s.is_none()).map(|s| s.lo),
    }
}

/// Whether `if (test)`, at `span`, tests an arm of a `match` of constants,
/// a `switch`, of `switches`, whose arms' values are no `?:` then (ADR 0629).
pub(super) fn switched<'a>(test: &Expr, span: js::Span, switches: impl IntoIterator<Item = &'a Switch>) -> bool {
    let mut arms = switches.into_iter().flat_map(|s| &s.arms);
    test_at(test, span).is_some_and(|at| arms.any(|arm| within(at, arm.pattern)))
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

/// A block's `break` in a `switch` its statements end with, `l: { if (c) {
/// .. } else { switch (k) { case 1: .. break l; } } }`, which goes where
/// the `switch`'s own `break` does, nothing between: that `break`, as a
/// person writes a case's end (ADR 0731); and the block's label, which
/// nothing names now, gone.
///
/// And an `Option`'s test of `None` before a `switch` of its value, `if (x
/// == null) { break l; } switch (x) { .. }` where they end the block, or
/// `if (x != null) { switch (x) { .. } }`, where no case is `null`'s and
/// the `default` does nothing: the `switch` alone, as JS's `switch` takes
/// what no case is, `switch (thenable.status) { case "fulfilled": .. }`, of
/// `match status.as_deref() { Some("fulfilled") => .., _ => {} }` (ADR
/// 0629, amended); a `default` of a `break` alone, nothing.
pub(super) fn breaks_past_switches(stmts: &mut Vec<Stmt>, switches: &[Switch]) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            let label = label.clone();
            ending_switches(body, &mut |case| case_breaks(case, &label));
            // `None`, which no case takes, goes past the `switch` to the
            // block's end, as the `break` does.
            if let [.., guard, switch] = &body[..]
                && let StmtKind::If(test, then, None) = &guard.kind
                && let [
                    Stmt {
                        kind: StmtKind::Break(Some(to)),
                        ..
                    },
                ] = &then[..]
                && *to == label
                && js::null_test(test, false).is_some_and(|x| switch_of(switch, x))
            {
                body.remove(body.len() - 2);
            }
        }
    }
    unlabeled(stmts);
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::If(test, then, None) = &stmts[i].kind
            && let [switch] = &then[..]
            && js::null_test(test, true).is_some_and(|x| switch_of(switch, x))
        {
            let switch = switch.clone();
            stmts[i] = switch;
        }
        super::case_rests::copied_returns_broken(stmts, i, switches);
        if let StmtKind::Switch { cases, default, .. } = &mut stmts[i].kind {
            if default.as_deref().is_some_and(does_nothing) {
                *default = None;
            }
            // A `break` that ends a branch of the `if` a case that doesn't
            // fall ends with, which the case's end does, as its `break;`
            // after it, or the `switch`'s end, does: `if (g) { .. }`.
            for body in (cases.iter_mut().filter(|case| !case.falls).map(|case| &mut case.body)).chain(default.as_mut())
            {
                ending_breaks(body);
                guard_held(body, switches);
            }
            // The last case's `break`, which its end does (ADR 0629,
            // amended), one that falls to nothing's too, ending its guard's
            // branch (#389c).
            if default.is_none()
                && let Some(last) = cases.iter_mut().rfind(|case| !case.values.is_empty())
            {
                if matches!(last.body.last().map(|s| &s.kind), Some(StmtKind::Break(None))) {
                    last.body.pop();
                }
                ending_breaks(&mut last.body);
            }
            if discriminant_in_place(stmts, i) {
                i -= 1;
            }
        }
        i += 1;
    }
}

/// Of `stmts`, a case's, each `break;` that ends a branch of the `if` they
/// end with, however deep, gone, which goes where that branch's end does;
/// a branch then empty, as `emptied_branch` writes it.
pub(super) fn ending_breaks(stmts: &mut Vec<Stmt>) {
    let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind) else {
        return;
    };
    for branch in std::iter::once(then).chain(otherwise) {
        if matches!(branch.last().map(|s| &s.kind), Some(StmtKind::Break(None))) {
            branch.pop();
        } else {
            ending_breaks(branch);
        }
    }
    let last = stmts.pop().expect("matched");
    if let Some(kind) = super::branches::emptied_branch(last.kind) {
        stmts.push(kind.at(last.span));
    }
}

/// Whether a `default`'s statements do nothing: none, or the `break` that
/// leaves the `switch`, which its end does.
fn does_nothing(stmts: &[Stmt]) -> bool {
    matches!(
        stmts,
        [] | [Stmt {
            kind: StmtKind::Break(None),
            ..
        }]
    )
}

/// Whether `stmt` is a `switch` of the variable `x`, each case a literal,
/// which `null` and `undefined` aren't, and a `default` that does nothing:
/// one that does nothing of what's nullish.
fn switch_of(stmt: &Stmt, x: &Expr) -> bool {
    let StmtKind::Switch {
        discriminant,
        cases,
        default,
    } = &stmt.kind
    else {
        return false;
    };
    let literal = |v: &Expr| {
        matches!(
            v.kind,
            ExprKind::Str(_) | ExprKind::Num(_) | ExprKind::Bool(_) | ExprKind::BigInt(_) | ExprKind::BigUint(_)
        )
    };
    matches!(&x.kind, ExprKind::Var(_))
        && js::same_path(discriminant, x)
        && default.as_deref().is_none_or(does_nothing)
        && (cases.iter()).all(|case| case.values.iter().all(literal))
}

/// A block a case ends with, through the branches of an `if` it ends with,
/// `case "object": if (..) { .. } else { desc: { .. break desc; .. } }`:
/// where it ends, the case does, so each of its `break`s, outside a loop or
/// `switch` of its own, is the case's `break;`, as a person leaves a case
/// early, and its label, which nothing names now, gone (ADR 0790). A block
/// the program labeled, `'desc: { .. }`, Rust's way to leave a `match`'s
/// arm early, reads as JS leaves a case.
///
/// An `if` the program wrote with no `else`, `if (c) { ..; return; } t`,
/// to whose `then` MIR gave one, `t`, as both went on to the block's end,
/// has none once the `then` leaves where the case does: `t` after it, as
/// written (ADR 0805), not the `else` of an `else if` it reads as, of the
/// arm before it, where the block was all that arm's `else` held.
pub(super) fn blocks_ending_cases(stmts: &mut [Stmt], elses: &[(js::Span, js::Span, js::Span)]) {
    fn ending(stmts: &mut Vec<Stmt>, elses: &[(js::Span, js::Span, js::Span)]) {
        match stmts.last_mut().map(|s| &mut s.kind) {
            Some(StmtKind::If(_, a, b)) => {
                ending(a, elses);
                if let Some(b) = b {
                    ending(b, elses);
                }
            }
            Some(StmtKind::Labeled(label, body)) => {
                let label = label.clone();
                case_breaks(body, &label);
                ending(body, elses);
            }
            _ => {}
        }
        unlabeled(stmts);
        unwritten_else(stmts, elses);
    }
    for stmt in stmts.iter_mut() {
        if let StmtKind::Switch { cases, default, .. } = &mut stmt.kind {
            for case in js::case_lists_mut(cases, default) {
                ending(case, elses);
            }
        }
    }
}

/// `stmts`' last `if`, `if (c) { a } else { b }`, as `blocks_ending_cases`
/// has it: of an `if` written with no `else` (`Written::elses`) whose `a`
/// leaves, `if (c) { a } b`. What `b` declares is a body's name of its
/// own, given apart from the rest, `name$1`, which preparation names
/// `name` again only where JS's scopes let it (ADR 0352).
pub(super) fn unwritten_else(stmts: &mut Vec<Stmt>, elses: &[(js::Span, js::Span, js::Span)]) {
    let Some(last) = stmts.last() else {
        return;
    };
    let StmtKind::If(_, a, Some(_)) = &last.kind else {
        return;
    };
    let written =
        |span: js::Span| span.is_none() || elses.iter().any(|(cond, ..)| cond.lo <= span.lo && span.hi <= cond.hi);
    if written(last.span) || !a.last().is_some_and(js::ends) {
        return;
    }
    let Some(Stmt {
        kind: StmtKind::If(_, _, b),
        ..
    }) = stmts.last_mut()
    else {
        unreachable!("matched")
    };
    let b = b.take().expect("matched");
    stmts.extend(b);
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
pub(super) fn case_breaks(stmts: &mut [Stmt], label: &str) {
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
