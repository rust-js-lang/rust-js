//! Tidy's chains of arms a variable's value is given by: each arm its own,
//! as the chain is written, but where every arm is a value alone, one
//! nested conditional (ADR 0969, amended); and a function's value of such
//! a chain, each arm's `return`, alike (`returned_else_ifs`).

use super::*;

/// Whether the `if` of `test`, at `span`, is one with an `else` the
/// program wrote as a statement, of a late `let`, `let error; if c { error
/// = a; } else { error = b; }`, which stays as it's written, its branches
/// no conditional (rd187, ADR 0969, amended).
pub(super) fn written_as_statement(test: &Expr, span: js::Span, program: &crate::lower::mir::hints::Written) -> bool {
    program.statement_elses.iter().any(|c| {
        let within = |s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
        within(test.span) || within(span)
    })
}

/// Of `x = t ? q : r`, the rest of a chain of arms a variable's value is
/// given by, folded, its arms as the chain writes them: `if (t) { x = q; }
/// else ..`, the chain going on where `r` is another, `else if (c) { x = r;
/// } else { x = s; }`, and of `!t ? e : r`, MIR's arms the other way, `if
/// (t) { x = r; } else { x = e; }`. Each arm a value alone, or the
/// conditional of an `if` in it, `wasHydrated ? a : b`; `None` of what's no
/// conditional.
pub(super) fn chained_arms(x: &str, value: &Expr, at: js::Span) -> Option<Vec<Stmt>> {
    let (t, q, r) = as_written(value)?;
    let arm_value = |e: &Expr| small(e) || matches!(&e.kind, ExprKind::Cond(_, a, b) if small(a) && small(b));
    let arm = |value: &Expr| vec![StmtKind::Assign(Expr::var(x), value.clone()).at(at)];
    let rest = match chained_arms(x, r, at) {
        Some(rest) => rest,
        None if arm_value(r) => arm(r),
        None => return None,
    };
    arm_value(q).then(|| vec![StmtKind::If(t.clone(), arm(q), Some(rest)).at(at)])
}

/// Of the rest of a chain of arms, folded, `x = b ? q : c ? r : s`, whose
/// every arm is a plain value (`plain`): that conditional, its arms as the
/// chain writes them, `!t ? e : r` of MIR's the other way round `t ? r :
/// e`, as upstream React writes its colors, `const color = a ? 'x' : b ?
/// 'y' : 'z';`. `None` where an arm is more, a call, an object or a
/// conditional of its own, whose chain stays `else if` (`chained_arms`).
pub(super) fn plain_chain(value: &Expr) -> Option<Expr> {
    let Some((t, q, r)) = as_written(value) else {
        return plain(value).then(|| value.clone());
    };
    let rest = plain_chain(r)?;
    plain(q).then(|| Expr {
        kind: ExprKind::Cond(Box::new(t.clone()), Box::new(q.clone()), Box::new(rest)),
        span: value.span,
        facts: value.facts,
    })
}

/// Of `if (t) { x = yes; } else { x = no; }`, one arm a plain value and the
/// other the rest of a chain of plain values, folded (`plain_chain`): the
/// one conditional, the plain value first, `t ? yes : b ? q : r`, and of
/// the chain in `yes`, `!t ? no : ..`, as `match o { None => .., Some(A)
/// => .., .. }` is `o == null ? .. : o === "A" ? .. : ..`. Of an `if` the
/// program wrote with an `else`, `turned` saying whether MIR turned its
/// branches (`written_order`), each where it's written: `t ? (b ? q : r) :
/// no` of a chain written in its `then`, React DOM's `blockedReason`. `None`
/// where neither arm or both are conditionals, a tree and no chain.
pub(super) fn plain_chain_of(test: &Expr, yes: Expr, no: Expr, turned: Option<bool>) -> Option<Expr> {
    let chain = |e: &Expr| matches!(e.kind, ExprKind::Cond(..));
    match (chain(&yes), chain(&no), turned) {
        (true, false, Some(false)) if plain(&no) => Some(Expr::cond(test.clone(), plain_chain(&yes)?, no)),
        (false, true, Some(true)) if plain(&yes) => {
            Some(Expr::cond(test.clone().negated_test(), plain_chain(&no)?, yes))
        }
        (false, true, _) if plain(&yes) => Some(Expr::cond(test.clone(), yes, plain_chain(&no)?)),
        (true, false, _) if plain(&no) => Some(Expr::cond(test.clone().negated_test(), no, plain_chain(&yes)?)),
        _ => None,
    }
}

/// Of a chain's conditional, `t ? q : r`, its test, its arm and the rest
/// of the chain, as the chain writes them: of `!t ? e : r`, MIR's arms the
/// other way round, `t`, `r` and `e`. `None` of what's no conditional.
fn as_written(value: &Expr) -> Option<(&Expr, &Expr, &Expr)> {
    let ExprKind::Cond(t, q, r) = &value.kind else {
        return None;
    };
    Some(match &t.kind {
        ExprKind::Unary(UnaryOp::Not, inner) => (&**inner, r, q),
        _ => (&**t, q, r),
    })
}

/// Whether `e` is a value alone, as a chain of plain values' arms are: a
/// string or a number literal, `-1` too, or a name.
pub(super) fn plain(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Str(_) | ExprKind::Num(_) | ExprKind::Var(_) => true,
        ExprKind::Unary(UnaryOp::Neg, n) => matches!(n.kind, ExprKind::Num(_)),
        _ => false,
    }
}

/// Of `let t; t = v; x = t;`, a temporary MIR gives a variable's value
/// through, declared there and read once: `x` and `v`, as `x = v;` is.
pub(super) fn assigned_through(stmts: &[Stmt]) -> Option<(&str, &Expr)> {
    let [
        Stmt {
            kind: StmtKind::Let(t, None),
            ..
        },
        Stmt {
            kind: StmtKind::Assign(made, value),
            ..
        },
        Stmt {
            kind: StmtKind::Assign(target, read),
            ..
        },
    ] = stmts
    else {
        return None;
    };
    match (&made.kind, &target.kind, &read.kind) {
        (ExprKind::Var(m), ExprKind::Var(x), ExprKind::Var(r)) if is_temporary(t) && m == t && r == t => {
            Some((x.as_str(), value))
        }
        _ => None,
    }
}

/// The value of an `else if` chain `stmts` end with, the user's rule of a
/// variable's (`plain_chain_of`) for a function's (React's rd23): its arms
/// each a plain value (`plain`) its `return` gives, none the program wrote
/// but the chain's own, `return if a { x } else if ..;`, one nested
/// conditional, `return a ? x : b ? y : z;`, each value where its arm is;
/// and otherwise the chain as written, its last link's two arms, which
/// `returned_values` folded alone, `else { return b ? y : z; }`, half a
/// chain and half a conditional, its own again: `else if (b) { return y; }
/// else { return z; }`. Not after a link an `else if let` binds first, a
/// `const`, where JS's chain is no more (`written_elses`) and its links are
/// each an `if` of its own.
pub(super) fn returned_else_ifs(stmts: &mut Vec<Stmt>, links: &[js::Span], returns: &[js::Span]) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    // A link of a chain, an `else if`, at its condition, where MIR puts
    // its `if`, and the `return` `returned_values` folded it into.
    let link = |s: &Stmt| links.iter().any(|l| within(l, s.span));
    let Some(mut head) = stmts.iter().rposition(|s| matches!(s.kind, StmtKind::If(..))) else {
        return;
    };
    // The chain's head, of links each an `if` after the last, where each
    // leaves.
    let guard = |s: &Stmt| matches!(&s.kind, StmtKind::If(_, then, None) if then.last().is_some_and(ends));
    while head > 0 && guard(&stmts[head - 1]) && link(&stmts[head]) {
        head -= 1;
    }
    let span = stmts[head].span;
    // A plain value its `return` gives, MIR's, or the one the program wrote
    // of the chain.
    let valued = |s: &Stmt| match &s.kind {
        StmtKind::Return(Some(value))
            if plain(value)
                && (!written(returns, s.span) || returns.iter().any(|r| within(r, s.span) && within(r, span))) =>
        {
            Some(spanned(value, s.span))
        }
        _ => None,
    };
    // The chain's links in turn, each its test and its arm, and the last
    // `return` of the chain, a link's two folded.
    let mut tested: Vec<(&Stmt, &Expr, &[Stmt])> = Vec::new();
    let mut list = &stmts[head..];
    let last = loop {
        match list {
            [
                link @ Stmt {
                    kind: StmtKind::If(test, then, Some(otherwise)),
                    ..
                },
            ] => {
                tested.push((link, test, then));
                list = otherwise;
            }
            [
                link @ Stmt {
                    kind: StmtKind::If(test, then, None),
                    ..
                },
                rest @ ..,
            ] if then.last().is_some_and(ends) => {
                tested.push((link, test, then));
                list = rest;
            }
            [
                last @ Stmt {
                    kind: StmtKind::Return(Some(_)),
                    ..
                },
            ] => break last,
            _ => return,
        }
    };
    // The head no link, which is a chain's of its own, or the link an
    // `else if let` binds just before, `const s = ..; if (s != null)`, after
    // which JS's chain is no more and the rest a chain of its own (the
    // user's rule, pt3, rd23b); each link after it an `else if` the program
    // wrote.
    let bound = head > 0
        && matches!(&stmts[head - 1].kind, StmtKind::Const(name, _) | StmtKind::Let(name, Some(_))
            if tested[0].1.mentions_var(name));
    if link(tested[0].0) && !bound || !tested[1..].iter().all(|(s, ..)| link(s)) {
        return;
    }
    let StmtKind::Return(Some(value)) = &last.kind else {
        unreachable!("matched")
    };
    let folded = match &value.kind {
        ExprKind::Cond(test, yes, no) if link(last) => Some((&**test, &**yes, &**no)),
        _ => None,
    };
    let arms = tested.len() + 1 + usize::from(folded.is_some());
    if arms < 3 {
        return;
    }
    let mut values: Option<Vec<(Expr, Expr)>> = (tested.iter())
        .map(|(_, test, then)| match then {
            [arm] => valued(arm).map(|v| ((*test).clone(), v)),
            _ => None,
        })
        .collect();
    let rest = match folded {
        Some((test, yes, no)) => {
            if let Some(values) = &mut values
                && plain(yes)
                && plain(no)
            {
                values.push((test.clone(), yes.clone()));
            }
            no.clone()
        }
        None => valued(last).unwrap_or_else(|| value.clone()),
    };
    if let Some(values) = values.filter(|v| v.len() == arms - 1)
        && plain(&rest)
    {
        let value = (values.into_iter().rev()).fold(rest, |no, (test, yes)| Expr::cond(test, yes, no));
        stmts.truncate(head);
        stmts.push(StmtKind::Return(Some(value)).at(span));
        return;
    }
    // The chain as written: its last link's arms each its own.
    let Some((test, yes, no)) = folded else {
        return;
    };
    let at = last.span;
    let ret = |v: &Expr| vec![StmtKind::Return(Some(v.clone())).at(at)];
    let test = Expr {
        facts: test.facts.with(js::Known::Else),
        ..test.clone()
    };
    let unfolded = StmtKind::If(test, ret(yes), Some(ret(no))).at(at);
    // In the link before's `else`, which the program wrote, where that's
    // a guard still, as `written_elses` leaves one of two returns alone.
    fn place(list: &mut Vec<Stmt>, unfolded: Stmt) {
        match list.as_mut_slice() {
            [
                Stmt {
                    kind: StmtKind::If(_, _, Some(otherwise)),
                    ..
                },
            ] => place(otherwise, unfolded),
            [
                ..,
                Stmt {
                    kind: StmtKind::If(test, _, otherwise @ None),
                    ..
                },
                _,
            ] => {
                test.facts = test.facts.with(js::Known::Else);
                *otherwise = Some(vec![unfolded]);
                list.pop();
            }
            [.., last] => *last = unfolded,
            [] => unreachable!("walked"),
        }
    }
    let mut chain = stmts.split_off(head);
    place(&mut chain, unfolded);
    stmts.extend(chain);
}
