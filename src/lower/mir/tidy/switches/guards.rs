//! A guarded arm's case: the guard's `if`, where its guard fails going on
//! as Rust goes on to the next arm (ADR 0905), and MIR's block of a guarded
//! `|`, the arm's one test (rd136).

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind};
use crate::lower::mir::hints::{Guard, Switch, SwitchArm};

use super::{Case, alike, test_at, within, written_at};

/// A block MIR made of a guarded or-pattern's arm, whose guard calls, which
/// `decisions` leaves the `switch`: each alternative tested in turn, the
/// guard in each, failing past the block, as nothing else does, `bb1: { if
/// (name === "a") { if (!on(name)) { break bb1; } } else if (name === "b")
/// { if (!on(name)) { break bb1; } } else { break bb1; } return; }`, and
/// what the arm does then, which leaves: `if ((name === "a" || name ===
/// "b") && on(name)) { return; }`, the arm's one test, as `A | B if
/// on(name)` is written, which `guarded` reads as its pattern's and its
/// guard's (rd136). The alternatives compare what's matched, so one holds
/// at most, and the guard runs where one does, once, as it did.
pub(super) fn guards_once(list: &mut [Stmt], switches: &[Switch]) {
    for stmt in list.iter_mut() {
        let span = stmt.span;
        let StmtKind::Labeled(label, body) = &mut stmt.kind else {
            continue;
        };
        if !crate::lower::mir::is_block_label(label) {
            continue;
        }
        let [chain, tail @ ..] = &body[..] else { continue };
        if !tail.last().is_some_and(js::ends) || js::names_break(tail, label) {
            continue;
        }
        let broken = |stmts: &[Stmt]| matches!(stmts, [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == label);
        // Each alternative's test, and its guard's.
        let mut tested = Vec::new();
        let mut at = chain;
        let ends_broken = loop {
            let StmtKind::If(test, then, Some(otherwise)) = &at.kind else {
                break false;
            };
            let [
                Stmt {
                    kind: StmtKind::If(failed, broke, None),
                    ..
                },
            ] = &then[..]
            else {
                break false;
            };
            if !broken(broke) {
                break false;
            }
            tested.push((test, failed));
            match &otherwise[..] {
                rest if broken(rest) => break true,
                [next] => at = next,
                _ => break false,
            }
        };
        if !ends_broken || tested.len() < 2 {
            continue;
        }
        // Of one guarded arm of a `match` of `switches`: each test of its
        // pattern, each guard's of its guard, alike.
        let arm = (switches.iter().flat_map(|s| &s.arms)).find(|arm| {
            arm.guard.is_some() && test_at(tested[0].0, js::Span::NONE).is_some_and(|at| within(at, arm.pattern))
        });
        let Some((arm, guard)) = arm.and_then(|arm| Some((arm, arm.guard?))) else {
            continue;
        };
        let of = |e: &Expr, span: js::Span| test_at(e, js::Span::NONE).is_some_and(|at| within(at, span));
        let failed = tested[0].1;
        if !(tested.iter()).all(|(test, f)| of(test, arm.pattern) && of(f, guard) && alike(f, failed)) {
            continue;
        }
        let pattern = (tested.iter().map(|(test, _)| (*test).clone()))
            .reduce(|a, b| Expr::bin(Op::Or, a, b))
            .expect("two alternatives");
        let test = Expr::bin(Op::And, pattern, failed.clone().negated_test());
        let tail = tail.to_vec();
        stmt.kind = StmtKind::If(test, tail, None);
        stmt.span = span;
    }
}

/// Of `cases`, each in the order its arm is written, each of an arm that
/// falls to the next (`SwitchArm::falls`), whose statements are the
/// guard's `if` and then the next case's: the guard's `if` alone, its case
/// falling through to the next (ADR 0905). Whether each falls, or `None`
/// where one's statements aren't so. Where its guard fails, a case only
/// tested, then goes on to the next's statements, as Rust goes on to the
/// next arm; and where it holds, it leaves, as its arm's statements did,
/// a `break;` of the `switch` where they go on past the `match`.
pub(super) fn fallen(cases: &mut [Case], arms: &[&SwitchArm]) -> Option<Vec<bool>> {
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
/// Of `&&`, tested in parts, one test already, `if (a && b)`, React's
/// finishConcurrentRender's (`guard_ands`, ADR 0905, amended).
fn guard_alone(body: &[Stmt], next: &[Stmt]) -> Option<Stmt> {
    let (first, rest) = body.split_first()?;
    let StmtKind::If(test, then, otherwise) = &first.kind else {
        return None;
    };
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
        _ => return None,
    };
    let (test, then) = held;
    Some(StmtKind::If(test, then, None).at(first.span))
}

/// A case's `if (!guard) { break; } ..`, MIR's test of a guard whose
/// failing goes past the `match`, where what follows leaves: `if (guard) {
/// .. }`, as the arm is written, its case going on past the `switch` where
/// the guard fails (ADR 0905, amended); of `a || b`, tested in parts, one
/// test already, `if (!a && !b) { break; }` (`guard_ands`), `if (a || b) {
/// .. }`. Only of a guard written of the arm what follows is written in.
pub(super) fn guard_held(stmts: &mut Vec<Stmt>, switches: &[Switch]) {
    let [first, rest @ ..] = &stmts[..] else {
        return;
    };
    let StmtKind::If(test, then, None) = &first.kind else {
        return;
    };
    let broken = matches!(
        then.as_slice(),
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
        .find(|arm| (arm.guard).is_some_and(|guard| test_at(test, first.span).is_some_and(|at| within(at, guard))));
    // Past the comments above a last `_ => {}`, which `switches` put
    // before the jump the case ends with: after the `if`, where the case
    // goes on (`default_marks`, rd702).
    let mine = |s: &Stmt| arm.is_some_and(|arm| written_at(s).is_some_and(|at| within(at, arm.span)));
    let marks = |s: &Stmt| matches!(s.kind, StmtKind::Comments) && !mine(s);
    if !(rest.iter()).all(|s| mine(s) || marks(s)) {
        return;
    }
    let (after, held): (Vec<Stmt>, Vec<Stmt>) = rest.iter().cloned().partition(marks);
    let held = StmtKind::If(test.clone().negated_test(), held, None).at(first.span);
    *stmts = std::iter::once(held).chain(after).collect();
}

/// `if (a) { if (b) { .. } }`, `list`'s, of a guard MIR tests in parts,
/// `"B" if keep && more`, each test the guard's and neither `if` with an
/// `else`: one `if` of the guard, `if (keep && more) { .. }`, as written,
/// in a `switch`'s case or the `if` chain of a `match` that's none, any
/// `match`'s (`Written::guards`). Each
/// way on is the same, nothing between the tests (#530, ADR 0636, amended).
pub(super) fn guard_ands(list: &mut [Stmt], guards: &[Guard]) {
    let guard_of = |t: &Expr| {
        let at = test_at(t, js::Span::NONE)?;
        guards.iter().map(|guard| guard.span).find(|&guard| within(at, guard))
    };
    for stmt in list.iter_mut() {
        let StmtKind::If(test, then, None) = &mut stmt.kind else {
            continue;
        };
        loop {
            let last = match &test.kind {
                ExprKind::Binary(Op::And, _, last) => &**last,
                _ => &*test,
            };
            let Some(guard) = guard_of(last) else {
                break;
            };
            let [
                Stmt {
                    kind: StmtKind::If(inner, held, None),
                    ..
                },
            ] = then.as_slice()
            else {
                break;
            };
            if guard_of(inner) != Some(guard) {
                break;
            }
            let (inner, held) = (inner.clone(), held.clone());
            *test = Expr::bin(Op::And, test.clone(), inner);
            *then = held;
        }
    }
}
