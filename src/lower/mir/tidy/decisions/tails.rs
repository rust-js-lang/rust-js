//! An arm's own `match` whose arms leave the block of the `match` it's
//! an arm of (`tails`, C1, #523), and a block whose `break`s each end a
//! branch it ends with (`breaks_ended`).

use super::*;

/// Of the block `label`'s `list`, one it ends with, each tree that a list
/// it ends with ends with, from the first statement that leaves the block:
/// an arm's own `match`, `match (op, l, r) { .. }` of `Expr::Binary(op, l,
/// r) => { let l = ..; match (op, l, r) { .. } }`, each arm of which MIR
/// gives the outer `match`'s value, `value = ..; break bb195;`, and its
/// last arm `(op, a, b) => return Err(..)` past them. There, a `break` of
/// the block goes where the list's end does, so that tree is a block's of
/// its own, of nothing after it: its chain there, as long as it reads
/// again only what does nothing, as the user decided for a tree whose
/// label stays otherwise (C1, #523). The list with each so, inner first;
/// `None` where none is.
pub(super) fn tails(label: &str, list: &[Stmt], told: &Told) -> Option<Vec<Stmt>> {
    tails_in(label, list, told, true)
}

/// `tails` of `list`, the block's own where `whole`, whose tree from its
/// first statement `chain` tried already.
fn tails_in(label: &str, list: &[Stmt], told: &Told, whole: bool) -> Option<Vec<Stmt>> {
    let mut list = list.to_vec();
    let mut changed = false;
    if let Some(StmtKind::If(_, then, otherwise)) = list.last_mut().map(|s| &mut s.kind) {
        for branch in std::iter::once(then).chain(otherwise) {
            if let Some(made) = tails_in(label, branch, told, false) {
                *branch = made;
                changed = true;
            }
        }
    }
    let declared = (list.iter())
        .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
        .count();
    let first = (list.iter()).position(|s| names_break(std::slice::from_ref(s), label));
    if let Some(k) = first
        && !(whole && k == declared)
        && matches!(list[k].kind, StmtKind::If(..))
        && let Some((chain, _)) = chain((label, list[k].span), &list[k..], &[], told, true)
    {
        list.truncate(k);
        list.extend(chain);
        changed = true;
    }
    changed.then_some(list)
}

/// Whether a test of `body`'s `if`s is a guard's that calls (`Told::guards`).
pub(super) fn calls_in(body: &[Stmt], told: &Told) -> bool {
    let mut calls = false;
    js::each_list(body, &mut |list| {
        for stmt in list {
            if let StmtKind::If(test, ..) = &stmt.kind {
                let mut parts = Vec::new();
                leaf_tests(test, js::Span::NONE, &mut parts);
                calls |= (parts.iter())
                    .any(|(part, at)| !pure(part) && !at.is_none() && told.guards.iter().any(|g| within(*at, g.span)));
            }
        }
    });
    calls
}

/// `body`, the block `label`'s, without the `break`s of it that each end a
/// branch of the `if` it ends with, `if (x === 0) { level = 0; break bb29;
/// } else if (..) { level = 2; }`, where no other `break` of it is: `None`
/// where there's none or another. Of a tree whose guard calls, the chain
/// of an inner tree, which made again would read its calls' arms out of
/// the order they're written.
pub(super) fn breaks_ended(label: &str, body: &[Stmt]) -> Option<Vec<Stmt>> {
    fn ended(list: &mut Vec<Stmt>, label: &str) -> bool {
        match list.last_mut().map(|s| &mut s.kind) {
            Some(StmtKind::Break(Some(l))) if l == label => {
                list.pop();
                true
            }
            Some(StmtKind::If(_, then, otherwise)) => {
                let mut any = ended(then, label);
                if let Some(otherwise) = otherwise {
                    any |= ended(otherwise, label);
                }
                any
            }
            _ => false,
        }
    }
    let mut body = body.to_vec();
    (ended(&mut body, label) && !names_break(&body, label)).then_some(body)
}
