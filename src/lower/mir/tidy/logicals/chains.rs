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
/// A binding between tests that call, `if let Some(r) = ROOT.with_borrow(
/// Clone::clone) && swap_root()`, is set where it's read, `(r =
/// inProgressRoot) != null && swapRoot()` (`set_where_read`), its `let`
/// declared before the chain (B1, the user's decision), with `lets`: of a
/// block still there once the rest of tidy is done.
pub(in crate::lower::mir::tidy) fn chains_in_blocks(stmts: &mut Vec<Stmt>, lets: bool) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Labeled(label, body) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        if !crate::lower::mir::is_block_label(label) {
            i += 1;
            continue;
        }
        let mut hoisting = Hoisting {
            on: lets,
            names: Vec::new(),
        };
        // Of a binding sunk alone: a chain of none is `branch_values`', or
        // a flag's (`flag values`).
        let made = match chained(body, label, &mut hoisting) {
            Some((0, chain)) if chain != *body => Some(chain_if(&chain, label, stmts[i].span, lets)),
            _ => {
                hoisting.names.clear();
                match chain_within(body, label, &mut hoisting) {
                    true => {
                        let (label, mut body) = (label.clone(), body.clone());
                        fold_within(&mut body, &label, &mut hoisting, lets);
                        Some(body.pop().expect("chain_within's `if`"))
                    }
                    false => None,
                }
            }
        };
        let Some(made) = made else {
            i += 1;
            continue;
        };
        let declared: Vec<Stmt> = (hoisting.names.into_iter())
            .map(|name| StmtKind::Let(name, None).at(js::Span::NONE))
            .collect();
        let n = declared.len() + 1;
        stmts.splice(i..=i, declared.into_iter().chain([made]));
        i += n;
    }
}

/// `chains_in_blocks`' fold of `list`, `chain_within` it: the chain where
/// it is, in the innermost `if`, after the bindings before it.
fn fold_within(list: &mut [Stmt], label: &str, hoisting: &mut Hoisting, lets: bool) {
    let [
        Stmt {
            kind: StmtKind::If(_, inner, None),
            ..
        },
    ] = list
    else {
        unreachable!("chain_within")
    };
    let Some((k, chain)) = chained(inner, label, hoisting) else {
        return fold_within(inner, label, hoisting, lets);
    };
    inner.truncate(k);
    inner.push(chain_if(&chain, label, js::Span::NONE, lets));
}

/// The one `if` of `chain`, `chained`'s of a block `label` written at
/// `span`: its tests' `&&`, its body, and what's after it its `else`;
/// negated, for `written elses` to write as the program did, but `lets`',
/// late, after that pass, as the let chain's written, `if (a && b) { s }
/// else { t }`.
fn chain_if(chain: &[Stmt], label: &str, span: js::Span, lets: bool) -> Stmt {
    let Some((tests, mut otherwise, then)) = or_chain(chain, label) else {
        unreachable!("chained")
    };
    let mut test = (tests.into_iter())
        .rev()
        .reduce(|b, a| Expr::bin(Op::And, a, b))
        .expect("one");
    unchained_past(&mut test, &mut otherwise);
    match (then.is_empty(), lets) {
        (true, _) => StmtKind::If(test, otherwise, None),
        (false, true) => StmtKind::If(test, otherwise, Some(then)),
        (false, false) => StmtKind::If(test.negated_test(), then, Some(otherwise)),
    }
    .at(made_where(span, chain))
}

/// Of `list`, a block's: how many bindings its chain follows, and the
/// chain, `or_chain`'s, of a let chain's bindings between its tests, each
/// `const x = y;` of a variable, out of it (`hoisted`), where the tests call
/// nothing, so the variable reads the same before them. Those it follows
/// stay where they are, read once, a getter's too (rd639); those between
/// its tests are sunk, read again, of what reads the same (`rereadable`);
/// or, where the tests call, set where they're read (`set_in_tests`).
fn chained(list: &[Stmt], label: &str, hoisting: &mut Hoisting) -> Option<(usize, Vec<Stmt>)> {
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
        let mut called = |e: &Expr| found |= runs_code(e);
        test.each(&mut called);
        let k = (then.iter())
            .take_while(|s| matches!(&s.kind, StmtKind::Const(_, value) if rereadable(value)))
            .count();
        then[..k].iter().for_each(|s| {
            if let StmtKind::Const(_, value) = &s.kind {
                value.each(&mut called);
            }
        });
        found || matches!(&then[k..], [next] if calls(next))
    }
    let mut set = Vec::new();
    if let Some(first) = chain.first_mut() {
        match calls(first) {
            false => sunk(first, Vec::new()),
            true => {
                let mut asked = Hoisting {
                    on: hoisting.on,
                    names: hoisting.names.clone(),
                };
                set_in_tests(first, &mut asked);
                set = asked.names;
            }
        }
    }
    or_chain(&chain, label)?;
    if !set.is_empty() {
        hoisting.names = set;
    }
    Some((k, chain))
}

/// Of a chain whose tests call, `if (a != null) { const r = inProgressRoot;
/// if (r != null && swapRoot()) { s } }`, each binding at the start of a
/// `then` before the `if` it's all of but, set where that `if`'s test
/// reads it (`set_where_read`), where it may be declared before
/// the chain: `if (a != null) { if ((r = inProgressRoot) != null &&
/// swapRoot()) { s } }`, each `let` `hoisting`'s.
fn set_in_tests(stmt: &mut Stmt, hoisting: &mut Hoisting) {
    let StmtKind::If(_, then, None) = &mut stmt.kind else {
        return;
    };
    let k = (then.iter())
        .take_while(|s| matches!(s.kind, StmtKind::Const(..)))
        .count();
    if !matches!(
        &then[k..],
        [Stmt {
            kind: StmtKind::If(..),
            ..
        }]
    ) {
        return;
    }
    for at in (0..k).rev() {
        let StmtKind::Const(name, _) = &then[at].kind else {
            unreachable!("counted")
        };
        let name = name.clone();
        if hoisting.declared(&name).is_none() {
            return;
        }
        set_where_read(then, at);
    }
    set_in_tests(&mut then[0], hoisting);
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
        .take_while(|s| matches!(&s.kind, StmtKind::Const(_, value) if rereadable(value)))
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
fn chain_within(body: &[Stmt], label: &str, hoisting: &mut Hoisting) -> bool {
    let [
        Stmt {
            kind: StmtKind::If(_, inner, None),
            ..
        },
    ] = body
    else {
        return false;
    };
    let mut asked = Hoisting {
        on: hoisting.on,
        names: Vec::new(),
    };
    chained(inner, label, &mut asked).is_some() || chain_within(inner, label, hoisting)
}
