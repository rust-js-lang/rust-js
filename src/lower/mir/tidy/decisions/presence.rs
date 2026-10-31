//! A presence test around a chain each of whose tests says it: the chain
//! alone, its `else` what the test's failing went on to (bb11).

use super::*;

/// `l: { if (x != null) { if (x === A || x === B) { a } else if (x === C)
/// { c } else { break l; } rest } } e`, of a `match` of `Some`s of
/// constants whose other arm leaves, `_ => { ..; break; }`, `e` leaving:
/// each test of the chain says `x` is there, so where it isn't they all
/// fail, and the chain's `else`, where the block's `break` went, is where
/// the presence test's failing went too: `if (x === A || x === B) { a }
/// else if (x === C) { c } else { e } rest`, with no label. A test that
/// doesn't say it is the presence test's too, `x != null && x !== 3`, as
/// where it was, and a test that says it runs nothing (`says`). The
/// block's `let`s declared before its chain, `let level;`, before it.
pub(in crate::lower::mir::tidy) fn presence_said(stmts: &mut Vec<Stmt>, switches: &[Switch]) {
    let mut i = 0;
    while i < stmts.len() {
        if let Some(said) = said_at(stmts, i, switches) {
            stmts.splice(i.., said);
        }
        i += 1;
    }
}

/// `stmts` from `i` on, the block at `i` and what follows it, `e`, as the
/// chain alone, `presence_said`'s, where they're of that shape.
fn said_at(stmts: &[Stmt], i: usize, switches: &[Switch]) -> Option<Vec<Stmt>> {
    let StmtKind::Labeled(label, body) = &stmts[i].kind else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::If(present, then, None),
            ..
        },
    ] = &body[..]
    else {
        return None;
    };
    let ExprKind::Binary(Op::LooseNe, subject, none) = &present.kind else {
        return None;
    };
    let after = &stmts[i + 1..];
    if !matches!(none.kind, ExprKind::Null | ExprKind::Undefined)
        || !after.last().is_some_and(ends)
        || names_break(after, label)
    {
        return None;
    }
    // Its variables declared first, `let level;`, declared before the chain,
    // where nothing before the block in its list declares them too.
    let declared = (then.iter())
        .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
        .count();
    let (lets, then) = then.split_at(declared);
    if (lets.iter()).any(|s| matches!(&s.kind, StmtKind::Let(name, _) if js::declares(&stmts[..i], name))) {
        return None;
    }
    // What the chain's arms go on to leaves, as what follows the block
    // does: neither runs after the other.
    let (chain, rest) = then.split_first()?;
    if names_break(rest, label) || !rest.last().is_some_and(ends) {
        return None;
    }
    let valued = |e: &Expr| valued(e, switches);
    // Each link of the chain, its test the presence test's too where it
    // doesn't say it, `x != null && x !== 3`, and its last `else`, the
    // block's `break` alone, what follows the block; no other way to the
    // block's end.
    let mut chain = chain.clone();
    let mut link = &mut chain;
    loop {
        let StmtKind::If(test, then, Some(otherwise)) = &mut link.kind else {
            return None;
        };
        // A link where a guard's failing goes on, `else if
        // (!Number.isNaN(x)) { break l; } else { a }` of `Some(x) if
        // x.is_nan() => a`: its test as the guard's, `else if
        // (Number.isNaN(x)) { a } else { break l; }`, the last link then.
        if matches!(&then[..], [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == label) {
            *test = std::mem::replace(test, Expr::undefined()).negated_test();
            std::mem::swap(then, otherwise);
        }
        if names_break(then, label) {
            return None;
        }
        if !says(subject, test, &valued) {
            *test = Expr::bin(Op::And, present.clone(), std::mem::replace(test, Expr::undefined()));
        }
        match &mut otherwise[..] {
            [
                Stmt {
                    kind: StmtKind::If(..), ..
                },
            ] => link = &mut otherwise[0],
            [
                Stmt {
                    kind: StmtKind::Break(Some(l)),
                    ..
                },
            ] if l == label => {
                *otherwise = after.to_vec();
                break;
            }
            _ => return None,
        }
    }
    Some(
        (lets.iter().cloned())
            .chain([chain])
            .chain(rest.iter().cloned())
            .collect(),
    )
}
