//! A chain of `js::strict_equals` tests of one subject, a `switch` (ADR
//! 0958, the user's decision): React's `switch (selector.$$typeof) { case
//! COMPONENT_TYPE: .. }` of selector tags that are JS values, which no Rust
//! `match` of constants can name.

use std::borrow::Cow;

use super::*;

/// How many cases make a `switch`, its `default` one: two are the `if` and
/// `else` a person writes of two ways, as ADR 0629's are.
const CASES: usize = 3;

/// Each `if (s === A) { .. } else if (s === B) { .. } else { .. }` of
/// `body`, each test a `js::strict_equals` the program wrote
/// (`strict_tests`) of one subject read the same each time, no getter, and
/// a value that does nothing: `switch (s) { case A: .. case B: .. default:
/// .. }`. JS's `switch` reads `s` once and each case's value in order until
/// one is `===` it, as the chain did. MIR's structure makes a guard of a
/// branch that never goes on, `if (s === A) { return a; } rest`, of an `if`
/// the program wrote with an `else` (`elses`) that `rest` is, to the list's
/// end; and `if (s !== C) { throw .. } rest` of `else if s === C { rest }
/// else { panic!() }`. A chain whose tests mix subjects stays, none of it a
/// `switch`, and so does one with a branch with a `break` of its own, which
/// a `switch` would take.
pub(super) fn strict_switches(
    body: &mut Vec<Stmt>,
    strict_tests: &[js::Span],
    elses: &[(js::Span, js::Span, js::Span)],
) {
    let mut mixed = Vec::new();
    js::each_list(body, &mut |stmts| {
        for at in 0..stmts.len() {
            let chain = chain_of(&stmts[at..], strict_tests, elses);
            if !chain.agrees() {
                mixed.extend(chain.tests);
            }
        }
    });
    js::statement_lists(body, &mut |stmts| {
        let mut at = 0;
        while at < stmts.len() {
            let chain = chain_of(&stmts[at..], strict_tests, elses);
            if chain.agrees()
                && !chain.tests.iter().any(|t| mixed.contains(t))
                && chain.cases.len() + usize::from(chain.default.is_some()) >= CASES
                && !chain.bodies().any(|b| breaks(b, None))
            {
                let switch = StmtKind::Switch {
                    discriminant: chain.subjects[0].clone(),
                    cases: (chain.cases.into_iter())
                        .map(|(value, body)| js::Case {
                            values: vec![value.clone()],
                            body: body.into_owned(),
                            falls: false,
                        })
                        .collect(),
                    default: chain.default.map(Cow::into_owned),
                };
                let (consumed, span) = (chain.consumed, stmts[at].span);
                stmts.splice(at..at + consumed, [switch.at(span)]);
            }
            at += 1;
        }
    });
}

/// An `if` chain of `js::strict_equals` tests, as a `switch`'s parts.
struct Chain<'a> {
    subjects: Vec<&'a Expr>,
    tests: Vec<js::Span>,
    /// Each case's value and statements.
    cases: Vec<(&'a Expr, Cow<'a, [Stmt]>)>,
    default: Option<Cow<'a, [Stmt]>>,
    /// How many statements of the list it is: its first, or each to the
    /// list's end, of the guards MIR's structure made.
    consumed: usize,
}

impl Chain<'_> {
    fn agrees(&self) -> bool {
        self.subjects.iter().all(|s| js::same_path(self.subjects[0], s))
    }

    fn bodies(&self) -> impl Iterator<Item = &[Stmt]> {
        (self.cases.iter())
            .map(|(_, body)| &**body)
            .chain(self.default.as_deref())
    }
}

/// The chain `stmts` start with, of no tests where they start otherwise.
fn chain_of<'a>(stmts: &'a [Stmt], strict_tests: &[js::Span], elses: &[(js::Span, js::Span, js::Span)]) -> Chain<'a> {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    // Where a statement is written: a `return`'s value where it is, of the
    // one return of a function's value, which MIR's structure moved into
    // each branch.
    let at = |s: &Stmt| match &s.kind {
        StmtKind::Return(Some(value)) if !value.span.is_none() => value.span,
        _ => s.span,
    };
    // Statements all written in `branch`, one at least.
    let of = |branch: &js::Span, stmts: &[Stmt]| {
        stmts.iter().any(|s| within(branch, at(s))) && stmts.iter().all(|s| at(s).is_none() || within(branch, at(s)))
    };
    let mut chain = Chain {
        subjects: Vec::new(),
        tests: Vec::new(),
        cases: Vec::new(),
        default: None,
        consumed: 0,
    };
    let mut rest = stmts;
    chain.default = loop {
        // The last two ways, a returned `?:` of an `if` written with an
        // `else`, which `returned_values` made of a chain of values, in the
        // order written: its `===`'s value the case's, not a `!==`'s.
        if let [
            Stmt {
                kind: StmtKind::Return(Some(returned)),
                span,
            },
        ] = rest
            && let ExprKind::Cond(test, yes, no) = &returned.kind
            && let Some((s, value, true)) = tested(test, strict_tests)
            && (elses.iter()).any(|(cond, ..)| test.span.lo <= cond.lo && cond.hi == test.span.hi)
        {
            let ret = |value: &Expr| vec![StmtKind::Return(Some(value.clone())).at(*span)];
            chain.subjects.push(s);
            chain.tests.push(test.span);
            chain.cases.push((value, Cow::Owned(ret(yes))));
            break Some(Cow::Owned(ret(no)));
        }
        let [
            Stmt {
                kind: StmtKind::If(test, then, otherwise),
                span,
            },
            after @ ..,
        ] = rest
        else {
            break (!rest.is_empty()).then_some(Cow::Borrowed(rest));
        };
        let Some((s, value, positive)) = tested(test, strict_tests) else {
            break Some(Cow::Borrowed(rest));
        };
        let written = || elses.iter().find(|(cond, ..)| within(cond, *span));
        let ending = then.last().is_some_and(ends);
        let first = chain.consumed == 0;
        // Its case, and the statements the chain goes on in, or its
        // `default`; and how many of the list it is, where it's first.
        let (case, next, consumed) = match otherwise.as_deref() {
            // `else if`, of the chain the `switch` is, or the last `if` of
            // an `else`'s statements.
            Some(next) if positive && (first || after.is_empty()) => ((value, Cow::Borrowed(&then[..])), Ok(next), 1),
            None if positive && ending && written().is_some_and(|(_, t, o)| of(t, then) && of(o, after)) => {
                ((value, Cow::Borrowed(&then[..])), Ok(after), stmts.len())
            }
            None if !positive && ending && written().is_some_and(|(_, t, o)| of(o, then) && of(t, after)) => (
                (value, Cow::Borrowed(after)),
                Err(Some(Cow::Borrowed(&then[..]))),
                stmts.len(),
            ),
            None if positive && after.is_empty() => ((value, Cow::Borrowed(&then[..])), Err(None), 1),
            _ => break Some(Cow::Borrowed(rest)),
        };
        chain.subjects.push(s);
        chain.tests.push(test.span);
        chain.cases.push(case);
        if first {
            chain.consumed = consumed;
        }
        match next {
            Ok(next) => rest = next,
            Err(default) => break default,
        }
    };
    chain
}

/// A test's subject and value, and whether it's `===`, of a
/// `js::strict_equals` the program wrote, which ends where the test does.
fn tested<'a>(test: &'a Expr, strict_tests: &[js::Span]) -> Option<(&'a Expr, &'a Expr, bool)> {
    let (positive, eq) = match &test.kind {
        ExprKind::Unary(js::UnaryOp::Not, inner) => (false, &**inner),
        _ => (true, test),
    };
    let (positive, s, value) = match &eq.kind {
        ExprKind::Binary(Op::Eq, s, value) => (positive, s, value),
        ExprKind::Binary(Op::Ne, s, value) if positive => (false, s, value),
        _ => return None,
    };
    let strict = [test.span, eq.span]
        .iter()
        .any(|e| !e.is_none() && strict_tests.iter().any(|t| e.lo <= t.lo && t.hi == e.hi));
    (strict && s.reads_same() && !value.has_effects()).then_some((&**s, &**value, positive))
}
