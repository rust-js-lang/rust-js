//! A block MIR made whose `break`s jump on to what follows it, its
//! continuation, past what the block's other ways go on to first: `l: { if
//! (c) { if (Continue) { acc = a; break l; } r = b; } <drops>; return r; }
//! <the rest>`, a `?`'s `Continue` in a `try_fold`. Each other way given
//! what it went on to, `if (Continue) { acc = a; } else { r = b; <drops>;
//! return r; }`, as a person writes it, each `break` last on its way to the
//! block's end then, and gone with the block's name (#2215).

use super::*;

/// Each MIR-made block of `stmts` whose `break`s `lead_on` makes last on
/// their ways to its end, each dropped then (`drop_tail`), and the block
/// named by nothing, which `unlabeled` takes away. Only where every one of
/// its `break`s goes so: a block that keeps one keeps its shape. Not of a
/// block the program labeled, whose `break`s it wrote.
pub(in crate::lower::mir::tidy) fn breaks_lead_on(stmts: &mut [Stmt], written: &crate::lower::mir::hints::Written) {
    for stmt in stmts {
        let StmtKind::Labeled(label, body) = &mut stmt.kind else {
            continue;
        };
        if !label.is_block_label() || written.blocks.iter().any(|block| label.is(&block.label)) {
            continue;
        }
        let mut led = body.clone();
        if lead_on(&mut led, label) {
            drop_tail(&mut led, &StmtKind::Break(Some(label.clone())));
            if !js::names_jump(&led, label) {
                *body = led;
            }
        }
    }
}

/// `stmts`, a list on the way of a `break label`, the innermost first,
/// each `if` holding one followed by nothing then: what followed it, `rest`,
/// moved into its branch that isn't the `break`'s, where the `break`'s never
/// goes on (`js::ends`) and holds no `break` of it, as nothing is copied,
/// and `rest` goes on, not a guard's; or written at the end of each of its
/// ways that goes on, where `rest` is a tail a `break` may be given
/// (`epilogues::copied_tail`), drops and a `return`. Not where the branch
/// it's written in declares a name `rest` reads, which it would read
/// instead, or `rest` declares one read before it. Whether it may be so:
/// a `break` in a `try` or a loop, which no `if` leads on past, is still
/// there after, and the block keeps its shape (`breaks_lead_on`).
fn lead_on(stmts: &mut Vec<Stmt>, label: &js::Name) -> bool {
    for stmt in stmts.iter_mut() {
        if let StmtKind::If(_, then, otherwise) = &mut stmt.kind
            && (!lead_on(then, label) || !otherwise.as_mut().is_none_or(|otherwise| lead_on(otherwise, label)))
        {
            return false;
        }
    }
    let Some(at) = (stmts.iter())
        .position(|s| matches!(s.kind, StmtKind::If(..)) && js::names_jump(std::slice::from_ref(s), label))
    else {
        return true;
    };
    if at + 1 == stmts.len() {
        return true;
    }
    let rest = stmts.split_off(at + 1);
    let read_before =
        |rest: &[Stmt], before: &[Stmt]| bindings::declared_names(rest).any(|name| js::mentions_of(before, name) > 0);
    if read_before(&rest, stmts) {
        return false;
    }
    let StmtKind::If(_, then, otherwise) = &mut stmts[at].kind else {
        unreachable!("found")
    };
    // Not where `rest` never goes on itself, `if (c) { break l; } s;
    // return x;`, a guard as `branch_ends` leaves one (ADR 0745), which a
    // `let .. else` or a `switch`'s case is written as.
    let guards = rest.last().is_some_and(js::ends);
    let leaves = |branch: &[Stmt]| !guards && js::names_jump(branch, label) && branch.last().is_some_and(js::ends);
    let empty = Vec::new();
    let other = otherwise.as_deref().unwrap_or(&empty);
    if leaves(then) && !js::names_jump(other, label) {
        if shadowed(other, &rest) {
            return false;
        }
        let other = otherwise.get_or_insert_with(Vec::new);
        other.extend(rest);
        return lead_on(other, label);
    }
    if leaves(other) && !js::names_jump(then, label) {
        if shadowed(then, &rest) {
            return false;
        }
        then.extend(rest);
        return lead_on(then, label);
    }
    if !super::epilogues::copied_tail(&rest) || shadowed(&stmts[at..=at], &rest) {
        return false;
    }
    given_at_ends(&mut stmts[at], &rest, label);
    true
}

/// Whether `stmts` declare a name `rest` reads, or one `rest` declares, in
/// any list of theirs `rest` would be written at the end of.
fn shadowed(stmts: &[Stmt], rest: &[Stmt]) -> bool {
    let mut declared = Vec::new();
    js::each_list(stmts, &mut |list| {
        declared.extend(bindings::declared_names(list).cloned())
    });
    declared.iter().any(|name| js::mentions_of(rest, name) > 0)
}

/// `rest` written at the end of each way through `stmt`, an `if` holding a
/// `break label`, that goes on: each branch's end, past an `if` that holds
/// one in turn, and an `else` of it where there's none.
fn given_at_ends(stmt: &mut Stmt, rest: &[Stmt], label: &js::Name) {
    let StmtKind::If(_, then, otherwise) = &mut stmt.kind else {
        unreachable!("an if")
    };
    let end = |branch: &mut Vec<Stmt>| match branch.last_mut() {
        Some(last) if js::ends(last) => {}
        Some(last) if matches!(last.kind, StmtKind::If(..)) && js::names_jump(std::slice::from_ref(last), label) => {
            given_at_ends(last, rest, label)
        }
        _ => branch.extend(rest.iter().cloned()),
    };
    end(then);
    end(otherwise.get_or_insert_with(Vec::new));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_js_program::js::{Expr, Known, Name, Span};

    fn stmt(kind: StmtKind) -> Stmt {
        kind.at(Span::NONE)
    }

    /// `dropF?.(f);`, a drop the drops module made, which may throw.
    fn drop() -> Stmt {
        stmt(StmtKind::Expr(Expr::optional_call(
            Expr::var("dropF"),
            vec![Expr::var("f")],
        )))
        .knowing(Known::Drop)
    }

    fn assign(to: &str, value: &str) -> Stmt {
        stmt(StmtKind::Assign(Expr::var(to), Expr::var(value)))
    }

    fn returned() -> Stmt {
        stmt(StmtKind::Return(Some(Expr::var("result"))))
    }

    /// `l: { if (c) { if (none) { result = init; } else { if (go) { acc =
    /// a; break l; } result = b; } <tail> } acc = init; } return acc;`, a
    /// `?`'s `Continue` in a `try_fold` of a `Peekable`.
    fn block(label: &Name, tail: Vec<Stmt>) -> Vec<Stmt> {
        let go = stmt(StmtKind::If(
            Expr::var("go"),
            vec![assign("acc", "a"), stmt(StmtKind::Break(Some(label.clone())))],
            None,
        ));
        let mut then = vec![stmt(StmtKind::If(
            Expr::var("none"),
            vec![assign("result", "init")],
            Some(vec![go, assign("result", "b")]),
        ))];
        then.extend(tail);
        vec![
            stmt(StmtKind::Labeled(
                label.clone(),
                vec![stmt(StmtKind::If(Expr::var("c"), then, None)), assign("acc", "init")],
            )),
            stmt(StmtKind::Return(Some(Expr::var("acc")))),
        ]
    }

    fn after(mut stmts: Vec<Stmt>) -> Vec<Stmt> {
        breaks_lead_on(&mut stmts, &crate::lower::mir::hints::Written::default());
        stmts
    }

    // A `then` that never goes on but by the `break` has what followed it
    // in its `else`; a tail of drops and a `return` is written at each other
    // way's end; each `break` goes (#2215).
    #[test]
    fn breaks_lead_on_to_the_continuation() {
        let l = Name::block_label("bb12".into());
        let stmts = after(block(&l, vec![drop(), returned()]));
        assert!(!js::names_jump(&stmts, &l));
        let StmtKind::Labeled(_, body) = &stmts[0].kind else {
            panic!("the block")
        };
        // `if (c) { if (none) { result = init; dropF?.(f); return result; }
        // else if (go) { acc = a; } else { result = b; dropF?.(f); return
        // result; } } else { acc = init; }`
        let [
            Stmt {
                kind: StmtKind::If(_, then, Some(otherwise)),
                ..
            },
        ] = body.as_slice()
        else {
            panic!("one if")
        };
        assert!(otherwise == &vec![assign("acc", "init")]);
        let [
            Stmt {
                kind: StmtKind::If(_, none, Some(some)),
                ..
            },
        ] = then.as_slice()
        else {
            panic!("its if")
        };
        assert!(none == &vec![assign("result", "init"), drop(), returned()]);
        let [
            Stmt {
                kind: StmtKind::If(_, go, Some(other)),
                ..
            },
        ] = some.as_slice()
        else {
            panic!("the Continue's if")
        };
        assert!(go == &vec![assign("acc", "a")]);
        assert!(other == &vec![assign("result", "b"), drop(), returned()]);
    }

    // An `else` that never goes on but by the `break` has what followed it
    // in its `then`.
    #[test]
    fn an_else_that_breaks_leads_on_too() {
        let l = Name::block_label("bb12".into());
        let body = vec![
            stmt(StmtKind::If(
                Expr::var("c"),
                vec![assign("result", "b")],
                Some(vec![assign("acc", "a"), stmt(StmtKind::Break(Some(l.clone())))]),
            )),
            assign("acc", "init"),
        ];
        let stmts = after(vec![stmt(StmtKind::Labeled(l.clone(), body))]);
        assert!(!js::names_jump(&stmts, &l));
        let StmtKind::Labeled(_, body) = &stmts[0].kind else {
            panic!("the block")
        };
        let [
            Stmt {
                kind: StmtKind::If(_, then, Some(otherwise)),
                ..
            },
        ] = body.as_slice()
        else {
            panic!("one if")
        };
        assert!(then == &vec![assign("result", "b"), assign("acc", "init")]);
        assert!(otherwise == &vec![assign("acc", "a")]);
    }

    // The block keeps its `break`s where they'd not all go: a tail that
    // isn't one a `break` may be given, an assignment and a `return`; a `break` inside
    // a `try`; a `then` that goes on, whose `else` would take what follows
    // it from the way that goes on; a branch that declares a name what's
    // moved into it reads; a guard; a name read before what declares it;
    // a block the program labeled.
    #[test]
    fn breaks_keep_their_jump_where_a_move_would_differ() {
        let l = Name::block_label("bb12".into());
        let kept = |stmts: Vec<Stmt>| js::names_jump(&after(stmts), &l);
        assert!(kept(block(&l, vec![assign("x", "y"), returned()])));
        let tried = vec![stmt(StmtKind::TryCatch(
            vec![stmt(StmtKind::Break(Some(l.clone())))],
            Some("error".into()),
            vec![],
        ))];
        assert!(kept(vec![stmt(StmtKind::Labeled(
            l.clone(),
            vec![stmt(StmtKind::If(Expr::var("c"), tried, None)), assign("acc", "init")]
        ))]));
        let goes_on = vec![stmt(StmtKind::If(
            Expr::var("go"),
            vec![stmt(StmtKind::If(
                Expr::var("d"),
                vec![stmt(StmtKind::Break(Some(l.clone())))],
                None,
            ))],
            None,
        ))];
        let mut on = goes_on.clone();
        on.push(assign("result", "b"));
        assert!(kept(vec![stmt(StmtKind::Labeled(l.clone(), on))]));
        let shadow = vec![
            stmt(StmtKind::If(
                Expr::var("go"),
                vec![stmt(StmtKind::Break(Some(l.clone())))],
                Some(vec![stmt(StmtKind::Let(Name::from("result"), None))]),
            )),
            assign("acc", "result"),
        ];
        assert!(kept(vec![stmt(StmtKind::Labeled(l.clone(), shadow))]));
        // A tail written where a branch declares a name it reads, `f`.
        let mut shadowed = block(&l, vec![drop(), returned()]);
        let StmtKind::Labeled(_, body) = &mut shadowed[0].kind else {
            panic!("the block")
        };
        let StmtKind::If(_, then, _) = &mut body[0].kind else {
            panic!("its if")
        };
        let StmtKind::If(_, none, _) = &mut then[0].kind else {
            panic!("the none's if")
        };
        none.insert(0, stmt(StmtKind::Let(Name::from("f"), None)));
        assert!(kept(shadowed));
        // A guard, `if (go) { break l; } work(); return result;`, as a `let
        // .. else` or a `switch`'s case is written.
        let work = stmt(StmtKind::Expr(Expr::call(Expr::var("work"), vec![])));
        let guard = vec![
            stmt(StmtKind::If(
                Expr::var("go"),
                vec![stmt(StmtKind::Break(Some(l.clone())))],
                None,
            )),
            work,
            returned(),
        ];
        assert!(kept(vec![stmt(StmtKind::Labeled(l.clone(), guard))]));
        // `if (y) { break l; } const y = a;`: a name read before it's declared.
        let early = vec![
            stmt(StmtKind::If(
                Expr::var("y"),
                vec![stmt(StmtKind::Break(Some(l.clone())))],
                None,
            )),
            stmt(StmtKind::Const(Name::from("y"), Expr::var("a"))),
        ];
        assert!(kept(vec![stmt(StmtKind::Labeled(l.clone(), early))]));
        let program = Name::label("found");
        assert!(js::names_jump(
            &after(block(&program, vec![drop(), returned()])),
            &program
        ));
    }
}
