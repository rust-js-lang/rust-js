//! Tidy's let chains with an `else` in a block MIR made: the chain one
//! `if` of its tests, its bindings between them read in its body (ADR 0946,
//! amended; ADR 0842, amended).

use super::*;

/// `l: { if (c) { if (a) { if (b) { s; break l; } } t } }`, `branch_values`'
/// `&&` chain (`or_chain`) in `if`s the block is all of, where the block goes
/// on after them: `if (c) { if (a && b) { s } else { t } }`, a let chain's
/// with an `else` inside an `if` (ADR 0946, amended). After the passes that
/// name its temporaries and join its `if`s, which the chain's tests then are.
/// And the block's own chain, of a binding between its tests, `l: { if (a)
/// { const x = y?.v; if (x != null) { s; break l; } } t }`, which
/// `branch_values` doesn't see, its `if`s not one in another alone: `if (a
/// && y?.v != null) { const x = y?.v; s } else { t }`, not the nested `if`s
/// whose `else` only a label or a copy of what's after the block writes
/// (rd614 b, ADR 0842, amended).
pub(in crate::lower::mir::tidy) fn chains_in_blocks(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::Labeled(label, body) = &mut stmt.kind else {
            continue;
        };
        if !crate::lower::mir::is_block_label(label) {
            continue;
        }
        // Of a binding sunk alone: a chain of none is `branch_values`', or
        // a flag's (`flag values`).
        if let Some((0, chain)) = chained(body, label)
            && chain != *body
        {
            *stmt = chain_if(&chain, label, stmt.span);
            continue;
        }
        if !chain_within(body, label) {
            continue;
        }
        let (label, mut body) = (label.clone(), std::mem::take(body));
        fold_within(&mut body, &label);
        *stmt = body.pop().expect("chain_within's `if`");
    }
}

/// `chains_in_blocks`' fold of `list`, `chain_within` it: the chain where
/// it is, in the innermost `if`, after the bindings before it.
fn fold_within(list: &mut [Stmt], label: &str) {
    let [
        Stmt {
            kind: StmtKind::If(_, inner, None),
            ..
        },
    ] = list
    else {
        unreachable!("chain_within")
    };
    let Some((k, chain)) = chained(inner, label) else {
        return fold_within(inner, label);
    };
    inner.truncate(k);
    inner.push(chain_if(&chain, label, js::Span::NONE));
}

/// The one `if` of `chain`, `chained`'s of a block `label` written at
/// `span`: its tests' `&&`, its body, and what's after it its `else`.
fn chain_if(chain: &[Stmt], label: &str, span: js::Span) -> Stmt {
    let Some((tests, mut otherwise, then)) = or_chain(chain, label) else {
        unreachable!("chained")
    };
    let mut test = (tests.into_iter())
        .rev()
        .reduce(|b, a| Expr::bin(Op::And, a, b))
        .expect("one");
    unchained_past(&mut test, &mut otherwise);
    match then.is_empty() {
        true => StmtKind::If(test, otherwise, None),
        false => StmtKind::If(test.negated_test(), then, Some(otherwise)),
    }
    .at(made_where(span, chain))
}

/// Of `list`, a block's: how many bindings its chain follows, and the
/// chain, `or_chain`'s, of a let chain's bindings between its tests, each
/// `const x = y;` of a variable, out of it (`hoisted`), where the tests call
/// nothing, so the variable reads the same before them.
fn chained(list: &[Stmt], label: &str) -> Option<(usize, Vec<Stmt>)> {
    let k = (list.iter())
        .take_while(|s| matches!(&s.kind, StmtKind::Const(_, value) if read_of_a_path(value)))
        .count();
    let mut chain = list[k..].to_vec();
    // Whether the chain's tests, or the bindings between them, call
    // anything: not what its body does once they've held.
    fn calls(stmt: &Stmt) -> bool {
        let StmtKind::If(test, then, None) = &stmt.kind else {
            return false;
        };
        let mut found = false;
        let mut called = |e: &Expr| {
            found |= matches!(
                e.kind,
                ExprKind::Call(..) | ExprKind::New(..) | ExprKind::Assigned(..) | ExprKind::Await(_)
            )
        };
        test.each(&mut called);
        let k = (then.iter())
            .take_while(|s| matches!(&s.kind, StmtKind::Const(_, value) if read_of_a_path(value)))
            .count();
        then[..k].iter().for_each(|s| {
            if let StmtKind::Const(_, value) = &s.kind {
                value.each(&mut called);
            }
        });
        found || matches!(&then[k..], [next] if calls(next))
    }
    if let Some(first) = chain.first_mut()
        && !calls(first)
    {
        sunk(first, Vec::new());
    }
    or_chain(&chain, label).map(|_| (k, chain))
}

/// A let chain's bindings at the start of each `then` of `if (a) { const
/// x = y.v; if (x != null) { s } }`, `carried` those above: in its last
/// `then`, each test of one a test of what it reads, `if (a) { if (y.v !=
/// null) { const x = y.v; s } }`, read only where each test before it
/// held, as it was, and the same, as the tests call nothing (ADR 0946,
/// amended).
fn sunk(stmt: &mut Stmt, mut carried: Vec<Stmt>) {
    let StmtKind::If(_, then, None) = &mut stmt.kind else {
        return;
    };
    let k = (then.iter())
        .take_while(|s| matches!(&s.kind, StmtKind::Const(_, value) if read_of_a_path(value)))
        .count();
    if !matches!(
        &then[k..],
        [Stmt {
            kind: StmtKind::If(..),
            ..
        }]
    ) {
        then.splice(0..0, carried);
        return;
    }
    carried.extend(then.drain(..k));
    if let StmtKind::If(test, ..) = &mut then[0].kind {
        let read = |n: &str| {
            (carried.iter()).rev().find_map(|c| match &c.kind {
                StmtKind::Const(name, value) if name == n => Some(value.clone()),
                _ => None,
            })
        };
        if let Some(tested) = test.substitute(&read) {
            *test = tested;
        }
    }
    sunk(&mut then[0], carried);
}

/// Whether `body`, a block `label`'s, is an `if` of no `else` alone, of
/// such an `if` or of `or_chain`'s `&&` chain (`chained`), to the block's
/// end: the chain each `if` is all of the block within (`chains_in_blocks`).
fn chain_within(body: &[Stmt], label: &str) -> bool {
    matches!(body, [Stmt { kind: StmtKind::If(_, inner, None), .. }]
        if chained(inner, label).is_some() || chain_within(inner, label))
}
