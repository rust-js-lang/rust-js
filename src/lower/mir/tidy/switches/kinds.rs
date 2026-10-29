//! The values of a `switch (typeof value)`'s cases, a `match` of an
//! untagged enum's kinds (ADR 0668): what each test of a variant, wherever
//! MIR tests it, says of what `typeof` tells.

use crate::js::{Expr, ExprKind, Op, UnaryOp};
use crate::lower::mir::hints::{Constant, SUBJECT};
use crate::lower::untagged::TypeofCase;

use super::{Values, alike, alternatives, compared};

/// `test`, of `SUBJECT`, of `subject`.
pub(super) fn of_subject(test: &Expr, subject: &Expr) -> Expr {
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
pub(super) fn kinds_of(tests: &[&Expr], constants: &[Constant], discriminant: &mut Option<Expr>) -> Option<Values> {
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
