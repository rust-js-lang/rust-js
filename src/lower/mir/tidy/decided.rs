//! Tidy's decided tests: a test an earlier one decides, its branch taken
//! in its place (ADR 0564). Never that a value it tests is there, an
//! `Option`'s `??` or `$unwrap` dropped: MIR's `proved_present` owns that
//! (ADR 0620, amended, #1933), as test/architecture-tidy-presence.test.ts
//! holds.

use super::*;

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
pub(super) fn decided_tests(stmts: &mut Vec<Stmt>, changing: &IndexSet<js::Name>, written: &[js::Span]) {
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
            split_decided(otherwise, &test, written);
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

/// The `else` of `if (t)`, `otherwise`, that starts with `if (!t)` of the
/// same condition the program wrote, MIR's split of one test into its two
/// ways, a `match`'s of a pair, `if (a == null) { .. } else if (a != null &&
/// b != null)`: its `then` in its place, as the `else` says it holds
/// (#1972). Not an `if (!t)` of a condition of its own the program wrote,
/// which says what it says (ADR 0564, amended), nor one past anything that
/// may change what `t` reads, which isn't first in the `else`.
fn split_decided(otherwise: &mut Vec<Stmt>, test: &Expr, written: &[js::Span]) {
    let Some(Stmt {
        kind: StmtKind::If(t, ..),
        ..
    }) = otherwise.first()
    else {
        return;
    };
    let condition = |s: js::Span| super::logicals::written_if(written, s);
    let one_condition = !t.span.is_none() && condition(t.span).is_some() && condition(t.span) == condition(test.span);
    if !one_condition || !negates(t, test) || disturbs(t, test) {
        return;
    }
    let StmtKind::If(_, then, _) = otherwise.remove(0).kind else {
        unreachable!("matched")
    };
    otherwise.splice(0..0, then);
}

/// Whether `test` can be decided by an earlier reading of it: it does
/// nothing, isn't a constant, and reads no variable something assigns; or
/// reads through a field, which anything that does something disturbs
/// (`disturbs_any`), a closure's assignment too, as only a call runs it.
fn decidable(test: &Expr, changing: &IndexSet<js::Name>) -> bool {
    let mut stable = !test.has_effects() && !test.is_constant();
    if test.reads_only_vars() {
        test.visit_names(&mut |v| stable &= !changing.contains(v));
    }
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
            StmtKind::If(t, ..) if let Some(taken) = decided(t, test, holds).filter(|_| !wrote(t)) => {
                let StmtKind::If(_, then, otherwise) = stmts.remove(j).kind else {
                    unreachable!("matched")
                };
                let branch = match taken {
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
            _ => {
                if disturbs_any(std::slice::from_ref(&stmts[j]), test) {
                    return;
                }
                j += 1;
            }
        }
    }
}

/// Each `if` of `stmts` a branch of which its decided tests emptied, as
/// `branches::emptied_branch` writes it, `if (b === 1) {}` nothing: an
/// or-pattern's alternative no value reaches, `(_, _, 1) | (_, 1, 1)`'s
/// second past the first's `c === 1` (#694), or a guard that fails of
/// some of its alternatives, `if (v1 !== 2)` inside `if (v1 === 2)`
/// (#714). Of each list innermost first (`bottom_up`), so an `if` whose
/// branch held only such `if`s goes too, wherever the test that decided
/// them is.
pub(super) fn emptied_ifs(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if !settled(stmts, i) {
            i += 1;
        }
    }
}

/// The `if` at `stmts[j]`, if a branch of it is empty, as
/// `branches::emptied_branch` writes it: whether it went.
fn settled(stmts: &mut Vec<Stmt>, j: usize) -> bool {
    let StmtKind::If(_, then, otherwise) = &stmts[j].kind else {
        return false;
    };
    if !then.is_empty() && otherwise.as_ref().is_none_or(|o| !o.is_empty()) {
        return false;
    }
    let kind = std::mem::replace(&mut stmts[j].kind, StmtKind::Break(None));
    match super::branches::emptied_branch(kind) {
        Some(kind) => {
            stmts[j].kind = kind;
            false
        }
        None => {
            stmts.remove(j);
            true
        }
    }
}

/// Of `t`, a chain of `&&` and `||`, where `test` is known to hold, or to
/// fail: the chain without the parts of it `test` decides, `a ||
/// Array.isArray(v)` in an `else` of `Array.isArray(v)` its `a`; none where
/// no part of it is (ADR 0743). A part whose value is known is left out of
/// its chain only where it does nothing, as a test that's decided does not.
fn decided_part(t: &Expr, test: &Expr, holds: bool) -> Option<Expr> {
    fn fold(e: &Expr, test: &Expr, holds: bool, changed: &mut bool) -> Expr {
        let ExprKind::Binary(op @ (Op::And | Op::Or), a, b) = &e.kind else {
            return match decided(e, test, holds) {
                Some(taken) => {
                    *changed = true;
                    Expr::bool(taken).or_at(e.span)
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

/// What `t` is where `test` holds, or fails, `holds`: of `test` itself,
/// or its negation. And of a test of one value against a text, `x.TAG ===
/// "Continue"`, where it holds, `x` is that text, so a test of `x` against
/// another, `x.TAG === "Break"`, fails, and `x.TAG !== "Break"` holds: a
/// variant's tag tested inside a test of another variant (#1954). Of texts
/// alone: two numbers written apart may be one value, `0` and `-0`.
fn decided(t: &Expr, test: &Expr, holds: bool) -> Option<bool> {
    if alike(t, test) {
        return Some(holds);
    }
    if negates(t, test) {
        return Some(!holds);
    }
    let (subject, text, equal) = against_a_text(test)?;
    if equal != holds {
        return None;
    }
    let (of, other, equal) = against_a_text(t)?;
    (of.written_alike(subject) && other != text).then_some(!equal)
}

/// Whether `a` is the test `b` is, as written or as MIR made it.
fn alike(a: &Expr, b: &Expr) -> bool {
    same_test(a, b) || a.written_alike(b)
}

/// Whether `t` is `test`'s negation: `if (!t)` in the `else` of `if (t)`,
/// which `split_decided` takes as holding there. Never a test of another
/// text, which #1954's `decided` decides elsewhere: MIR's split of one
/// written test is of one value (#1972).
fn negates(t: &Expr, test: &Expr) -> bool {
    alike(&t.clone().negated_test(), test)
}

/// Of `x === "A"`, `x`, `"A"` and that it's `===`; of `x !== "A"`, the
/// same and that it's `!==`.
fn against_a_text(e: &Expr) -> Option<(&Expr, &str, bool)> {
    let ExprKind::Binary(op @ (Op::Eq | Op::Ne), subject, text) = &e.kind else {
        return None;
    };
    let ExprKind::Str(text) = &text.kind else {
        return None;
    };
    Some((subject, text, *op == Op::Eq))
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
    test.visit_names(&mut |v| vars.push(v.clone()));
    if vars.iter().any(|v| js::Writes::sets(stmts, v)) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `x.TAG === "<tag>"`, or `!==`.
    fn tag(tag: &str, equal: bool) -> Expr {
        let op = if equal { Op::Eq } else { Op::Ne };
        Expr::bin(op, Expr::member(Expr::var("x"), crate::js::TAG), Expr::str(tag))
    }

    /// Where `x.TAG === "A"` holds, a test of `x` against another text is
    /// decided; where it fails, `x` may be any other, so none is (#1954).
    #[test]
    fn another_text_is_decided_only_where_the_test_holds() {
        let a = tag("A", true);
        assert_eq!(decided(&tag("B", true), &a, true), Some(false));
        assert_eq!(decided(&tag("B", false), &a, true), Some(true));
        assert_eq!(decided(&tag("B", true), &a, false), None);
        assert_eq!(decided(&tag("B", false), &a, false), None);
        // Where `x.TAG !== "A"` fails, `x` is `A`.
        assert_eq!(decided(&tag("B", true), &tag("A", false), false), Some(false));
        assert_eq!(decided(&tag("B", true), &tag("A", false), true), None);
        // Of the same text, the test itself.
        assert_eq!(decided(&tag("A", true), &a, true), Some(true));
        assert_eq!(decided(&tag("A", true), &a, false), Some(false));
    }

    /// Of numbers, only the test itself: two written apart may be one
    /// value, `0` and `-0`, which `same_test` reads as one test.
    #[test]
    fn another_number_is_not_decided() {
        let n = |k: f64| Expr::bin(Op::Eq, Expr::var("n"), Expr::num(k));
        assert_eq!(decided(&n(1.0), &n(0.0), true), None);
        assert_eq!(decided(&n(-0.0), &n(0.0), true), Some(true));
    }

    /// MIR's split of one written test takes only its negation as holding
    /// in the `else`: a test of another text, which `decided` decides
    /// there, is no split of the same test (#1972, #1954).
    #[test]
    fn a_split_is_of_the_negation_alone() {
        assert!(negates(&tag("A", false), &tag("A", true)));
        assert!(!negates(&tag("B", true), &tag("A", false)));
        assert_eq!(decided(&tag("B", true), &tag("A", false), false), Some(false));
    }
}
