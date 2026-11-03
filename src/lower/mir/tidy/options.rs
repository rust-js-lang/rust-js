//! Tidy's `Option`s: one made where it's tested is its test (ADR 0910).

use super::*;

/// An `Option` made where it's tested, `const x = t ? v : undefined; if
/// (x) { a } else { b }`, of a value `v` that's there (`Known::Present`):
/// `if (t) { a } else { b }`, `x` read as `v` in `a`, as `if let Some(x) =
/// kind.object()` is upstream's `if (typeof value === "object" && value
/// !== null)` (ADR 0910). Its test is the `Option`'s presence, `x != null`,
/// true exactly where `t` holds, `v` being there; or `x == null`, of a
/// branch that ends, `x` read past it. `v`
/// is a variable nothing changes, so it's what `x` was wherever `x` is
/// read, a closure's later too; `x` is read nowhere else, nor declared in
/// what reads it, and it's read nowhere `v` is another, a closure's own
/// `value` that reads no `x` being none (rd514).
/// Past what's only declared between, `let typeofNextType;` of what the
/// test's branches set.
pub(super) fn tested_options(stmts: &mut Vec<Stmt>, changing: &HashSet<String>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let declared = |s: &Stmt| matches!(s.kind, StmtKind::Let(_, None));
        let at = i + 1 + stmts[i + 1..].iter().take_while(|s| declared(s)).count();
        if at < stmts.len()
            && let Some((tested, rest)) = tested_option(&stmts[i], &stmts[at], &stmts[at + 1..], changing)
        {
            stmts[at].kind = tested;
            if let Some(rest) = rest {
                stmts.splice(at + 1.., rest);
            }
            stmts.remove(i);
        }
        i += 1;
    }
}

/// `tested_options`'s rewrite of `made`, the `Option`'s `const`, and
/// `tested`, its test, before `rest`: the `if` of where `t` holds, and
/// `rest`, where it read `x`, of `v` in its place.
fn tested_option(
    made: &Stmt,
    tested: &Stmt,
    rest: &[Stmt],
    changing: &HashSet<String>,
) -> Option<(StmtKind, Option<Vec<Stmt>>)> {
    let StmtKind::Const(x, made) = &made.kind else {
        return None;
    };
    let ExprKind::Cond(test, value, none) = &made.kind else {
        return None;
    };
    let ExprKind::Var(v) = &value.kind else { return None };
    if !matches!(none.kind, ExprKind::Undefined) || !value.facts.has(js::Known::Present) || changing.contains(v) {
        return None;
    }
    let StmtKind::If(presence, then, otherwise) = &tested.kind else {
        return None;
    };
    let of_x = |e: &Expr| matches!(&e.kind, ExprKind::Var(o) if o == x);
    // `x != null`, where it's set, or `x == null`, where it isn't.
    let set = match (js::null_test(presence, true), js::null_test(presence, false)) {
        (Some(option), _) if of_x(option) => true,
        (_, Some(option)) if of_x(option) => false,
        _ => return None,
    };
    // What reads `x`, and what mustn't: of `if (!x) { b } rest`, `rest`,
    // past a `b` that ends, as a `let .. else`'s does.
    let (reads, unread): (&[Stmt], [&[Stmt]; 2]) = match (set, otherwise) {
        (true, _) => (then, [otherwise.as_deref().unwrap_or_default(), rest]),
        (false, None) if then.last().is_some_and(ends) => (rest, [then, &[]]),
        (false, _) => return None,
    };
    let read_elsewhere = unread.iter().any(|stmts| js::mentions_in(stmts, x) > 0) || test.mentions_var(x);
    if read_elsewhere || js::declares(reads, x) || js::read_where_other(reads, x, v) {
        return None;
    }
    let mut reads = reads.to_vec();
    js::each_expr_mut(&mut reads, &mut |e| {
        if of_x(e) {
            *e = (**value).clone();
        }
    });
    Some(match set {
        true => (StmtKind::If((**test).clone(), reads, otherwise.clone()), None),
        false => (
            StmtKind::If(test.clone().negated_test(), then.clone(), None),
            Some(reads),
        ),
    })
}

/// A `take()` of a place rust-js made, then tested, `const c = x.f; x.f =
/// undefined; if (c != null) { a } else { b }`: the place tested, and taken
/// where it's `Some`, `if (x.f != null) { const c = x.f; x.f = undefined; a
/// } else { b }`, as upstream writes it (ADR 0996). The write is marked so
/// by lowering (`Known::Taken`): the place is a path nothing but `c` reads
/// between, read twice the same, and where it's `None` writing `None` over
/// it changes nothing. `c` is read nowhere but `a`.
pub(super) fn taken_options(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 2 < stmts.len() {
        if let Some(tested) = taken_option(&stmts[i], &stmts[i + 1], &stmts[i + 2], &stmts[i + 3..]) {
            let span = stmts[i + 2].span;
            stmts.splice(i..i + 3, [tested.at(span)]);
        }
        i += 1;
    }
}

/// A `take()`'s write of `None` over a place rust-js made (`Known::Taken`)
/// the program writes again before anything could see it: React's Flight
/// client's `let listeners = match &mut *chunk.state.borrow_mut() {
/// Pending { value } | Blocked { value } => value.take(), _ => None };
/// *chunk.state.borrow_mut() = Initialized { value: text };`, whose
/// `chunk.value = null;` is written over by `chunk.value = text;` (rd524):
/// dropped, as upstream's `const listeners = chunk.value; chunk.status =
/// 'fulfilled'; chunk.value = text;` never writes it. The place is a path
/// nothing but a read of it sees (`js::taken_place`, a plain data property
/// or a variable), so of what runs between, only a read of its object, or
/// what may run or throw, a call's or a getter's, could see it: each
/// statement between, past the end of an `if`'s branch the write is in too,
/// writes a variable, or another of the object's fields, a constant or a
/// variable, its object named nowhere else. Case none (ADR 0262): no
/// program can tell the `None` was there.
pub(super) fn overwritten_takes(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let rest = &stmts[i + 1..];
        // Of the write itself, in this list; or of one ending a branch of
        // an `if`, past which this list goes on.
        let dropped = match &stmts[i].kind {
            StmtKind::Assign(target, none) if taken_over(target, none, &[], rest) => Some(None),
            StmtKind::If(_, then, otherwise) => {
                let ending = |branch: &[Stmt]| {
                    (0..branch.len()).rev().find(|&k| match &branch[k].kind {
                        StmtKind::Assign(target, none) => taken_over(target, none, &branch[k + 1..], rest),
                        _ => false,
                    })
                };
                match (ending(then), otherwise.as_deref().and_then(ending)) {
                    (Some(k), _) => Some(Some((true, k))),
                    (None, Some(k)) => Some(Some((false, k))),
                    (None, None) => None,
                }
            }
            _ => None,
        };
        match dropped {
            Some(None) => {
                stmts.remove(i);
                continue;
            }
            Some(Some((then, k))) => {
                if let StmtKind::If(_, branch, otherwise) = &mut stmts[i].kind {
                    let branch = match then {
                        true => branch,
                        false => otherwise.as_mut().expect("the branch it's in"),
                    };
                    branch.remove(k);
                }
                continue;
            }
            None => i += 1,
        }
    }
}

/// Whether `target = none`, a take's write (`Known::Taken`) of a field of a
/// variable, is written over before anything sees it, `after` it in its
/// branch and then `rest` (`overwritten_takes`).
fn taken_over(target: &Expr, none: &Expr, after: &[Stmt], rest: &[Stmt]) -> bool {
    let ExprKind::Member(object, key) = &target.kind else {
        return false;
    };
    let ExprKind::Var(root) = &object.kind else {
        return false;
    };
    if !none.facts.has(js::Known::Taken) || !js::taken_place(target) {
        return false;
    }
    // A value whose making runs nothing, of no mention of the object.
    let quiet = |value: &Expr| value.is_constant() || matches!(&value.kind, ExprKind::Var(name) if name != root);
    // Another field of the object written, or a variable.
    let between = |stmt: &Stmt| match &stmt.kind {
        StmtKind::Assign(written, value) if quiet(value) => match &written.kind {
            ExprKind::Var(name) => name != root,
            ExprKind::Member(of, other) => {
                other != key && matches!(&of.kind, ExprKind::Var(name) if name == root) && js::taken_place(written)
            }
            _ => false,
        },
        _ => false,
    };
    if !after.iter().all(between) {
        return false;
    }
    let past = rest.iter().take_while(|stmt| between(stmt)).count();
    matches!(rest.get(past).map(|stmt| &stmt.kind),
        Some(StmtKind::Assign(written, value)) if quiet(value) && js::same_path(written, target) && js::taken_place(written))
}

/// `taken_options`'s rewrite of `made`, `const c = place;`, `written`, the
/// take's `place = undefined;`, and `tested`, the `if` of `c`, before
/// `rest`.
fn taken_option(made: &Stmt, written: &Stmt, tested: &Stmt, rest: &[Stmt]) -> Option<StmtKind> {
    let StmtKind::Const(c, place) = &made.kind else {
        return None;
    };
    let StmtKind::Assign(target, none) = &written.kind else {
        return None;
    };
    if !none.facts.has(js::Known::Taken) || !js::same_path(place, target) {
        return None;
    }
    let StmtKind::If(test, then, otherwise) = &tested.kind else {
        return None;
    };
    let of_c = |e: &Expr| matches!(&e.kind, ExprKind::Var(o) if o == c);
    // `c != null`, where it's taken in `then`, or `c == null`, in the else.
    let set = match (js::null_test(test, true), js::null_test(test, false)) {
        (Some(option), _) if of_c(option) => true,
        (_, Some(option)) if of_c(option) => false,
        _ => return None,
    };
    let (some, none_branch) = match set {
        true => (then.as_slice(), otherwise.as_deref().unwrap_or_default()),
        false => (otherwise.as_deref()?, then.as_slice()),
    };
    if js::mentions_in(none_branch, c) > 0 || js::mentions_in(rest, c) > 0 {
        return None;
    }
    let mut taken = vec![made.clone(), written.clone()];
    taken.extend(some.iter().cloned());
    let test = Expr::presence(place.clone(), set);
    Some(match set {
        true => StmtKind::If(test, taken, otherwise.clone()),
        false => StmtKind::If(test, then.clone(), Some(taken)),
    })
}
