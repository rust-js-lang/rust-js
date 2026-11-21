//! A block MIR made whose `break`s go on to a short tail that ends the
//! function's list, `l: { ..; result = r; break l; .. } <drops>; return
//! result;`: each `break l` that tail, an early return as a person writes
//! it, `result = r; dropF?.(f); return result;`, and the block named by
//! nothing then, which `unlabeled` takes away (#2215).

use super::*;

/// Each MIR-made block of `stmts` followed by a tail that ends them, one
/// a `break` may be given (`copied_tail`): its `break`s are that tail, each
/// way out runs it once, as the block's end does. Not where a `break` is
/// inside a `try` in the block, whose `finally` or `catch` would then run
/// after the tail's drops, or around them, where it ran before them; nor
/// where the block declares a name the tail reads, which a copy inside it
/// would read instead; nor of a block the program labeled, whose `break`s
/// it wrote.
pub(in crate::lower::mir::tidy) fn breaks_take_their_tail(
    stmts: &mut [Stmt],
    written: &crate::lower::mir::hints::Written,
) {
    let Some(at) = stmts.iter().rposition(|s| matches!(s.kind, StmtKind::Labeled(..))) else {
        return;
    };
    let tail = stmts[at + 1..].to_vec();
    if !copied_tail(&tail) {
        return;
    }
    let StmtKind::Labeled(label, body) = &mut stmts[at].kind else {
        unreachable!("found")
    };
    if !label.is_block_label() || written.blocks.iter().any(|block| label.is(&block.label)) {
        return;
    }
    let label = label.clone();
    let breaks = |s: &Stmt| matches!(&s.kind, StmtKind::Break(Some(l)) if *l == label);
    let mut tried = false;
    let mut declared = Vec::new();
    js::each_list(body, &mut |list| {
        for s in list {
            if matches!(s.kind, StmtKind::Try(..) | StmtKind::TryCatch(..)) {
                js::each_list(std::slice::from_ref(s), &mut |inner| tried |= inner.iter().any(breaks));
            }
        }
        declared.extend(bindings::declared_names(list).cloned());
    });
    if tried || declared.iter().any(|name| js::mentions_of(&tail, name) > 0) {
        return;
    }
    js::statement_lists(body, &mut |list| {
        if list.iter().any(breaks) {
            *list = std::mem::take(list)
                .into_iter()
                .flat_map(|s| if breaks(&s) { tail.clone() } else { vec![s] })
                .collect();
        }
    });
}

/// Whether `tail`, what a way out of a block goes on to, may be written in
/// each way out instead, an early return as a person writes it: a `return`
/// after at most `SHARED` statements in all, each one tidy copies
/// (`copied_once`), drops among them that a `finally` may not hold, which
/// may throw (`drops::finally_holds`: those it may hold are the drop
/// scope's one `try { .. } finally { .. }` around the block, its `return`s
/// where they're written, a guard's); or `const`s of what reads alike
/// written twice (`reads_alike_twice`) and a `return`, `const a = fhi;
/// return [lo, a];`, each copy in its own block. Not a `return` alone, nor
/// assignments with no drop: other passes', `returns_past_blocks`' and
/// `returned`'s, which copy one only where it reads better.
pub(super) fn copied_tail(tail: &[Stmt]) -> bool {
    let Some((last, before)) = tail.split_last() else {
        return false;
    };
    if !matches!(last.kind, StmtKind::Return(_)) || tail.len() > super::decisions::SHARED {
        return false;
    }
    let drops: Vec<Stmt> = before
        .iter()
        .filter(|s| s.facts.has(js::Known::Drop))
        .cloned()
        .collect();
    let declared =
        |s: &Stmt| matches!(&s.kind, StmtKind::Const(_, value) if super::decisions::reads_alike_twice(value));
    if drops.is_empty() {
        !before.is_empty() && before.iter().all(declared)
    } else {
        !crate::lower::drops::finally_holds(&drops) && before.iter().all(super::decisions::copied_once)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::{Expr, Known, Name, Span};

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

    /// `l: { if (c) { result = a; <early> } result = b; } <tail>`.
    fn block(label: &Name, early: Vec<Stmt>, tail: Vec<Stmt>) -> Vec<Stmt> {
        let mut then = vec![stmt(StmtKind::Assign(Expr::var("result"), Expr::var("a")))];
        then.extend(early);
        let mut stmts = vec![stmt(StmtKind::Labeled(
            label.clone(),
            vec![
                stmt(StmtKind::If(Expr::var("c"), then, None)),
                stmt(StmtKind::Assign(Expr::var("result"), Expr::var("b"))),
            ],
        ))];
        stmts.extend(tail);
        stmts
    }

    fn breaks(stmts: &[Stmt], label: &Name) -> usize {
        let mut count = 0;
        js::each_list(stmts, &mut |list| {
            count += list
                .iter()
                .filter(|s| matches!(&s.kind, StmtKind::Break(Some(l)) if l == label))
                .count()
        });
        count
    }

    fn after(mut stmts: Vec<Stmt>) -> Vec<Stmt> {
        breaks_take_their_tail(&mut stmts, &crate::lower::mir::hints::Written::default());
        stmts
    }

    // Each `break` of a block MIR made is the tail it goes on to, drops that
    // may throw and a `return`, an early return with its drops (#2215).
    #[test]
    fn a_break_takes_its_tail_of_drops() {
        let l = Name::block_label("bb27".into());
        let returned = || stmt(StmtKind::Return(Some(Expr::var("result"))));
        let stmts = after(block(
            &l,
            vec![stmt(StmtKind::Break(Some(l.clone())))],
            vec![drop(), returned()],
        ));
        assert_eq!(breaks(&stmts, &l), 0);
        let StmtKind::Labeled(_, body) = &stmts[0].kind else {
            panic!("the block")
        };
        let StmtKind::If(_, then, _) = &body[0].kind else {
            panic!("its if")
        };
        assert!(then[1] == drop() && then[2] == returned());
    }

    // A tail of `const`s of what reads alike written twice and a `return`,
    // `const a = fhi; return [lo, a];`, each `break` takes too; not one of
    // a `const` of a call, nor a `return` alone (#2215).
    #[test]
    fn a_break_takes_a_tail_of_consts() {
        let l = Name::block_label("bb18".into());
        let returned = || stmt(StmtKind::Return(Some(Expr::var("a"))));
        let jump = || stmt(StmtKind::Break(Some(l.clone())));
        let read = stmt(StmtKind::Const(Name::from("a"), Expr::var("fhi")));
        assert_eq!(breaks(&after(block(&l, vec![jump()], vec![read, returned()])), &l), 0);
        let called = stmt(StmtKind::Const(Name::from("a"), Expr::call(Expr::var("f"), vec![])));
        assert_eq!(breaks(&after(block(&l, vec![jump()], vec![called, returned()])), &l), 1);
    }

    // Not past a `try` between the `break` and the block, whose handler
    // would run after the copy's drops, or around them; not where the block
    // declares a name the tail reads; not of drops a `finally` holds, the
    // drop scope's; not of a tail with no drop, other passes'.
    #[test]
    fn a_break_keeps_its_jump_where_a_copy_would_differ() {
        let l = Name::block_label("bb27".into());
        let returned = || stmt(StmtKind::Return(Some(Expr::var("result"))));
        let jump = || stmt(StmtKind::Break(Some(l.clone())));
        let tried = vec![stmt(StmtKind::TryCatch(vec![jump()], Some("error".into()), vec![]))];
        assert_eq!(breaks(&after(block(&l, tried, vec![drop(), returned()])), &l), 1);
        let shadowed = vec![stmt(StmtKind::Let(Name::from("f"), None)), jump()];
        assert_eq!(breaks(&after(block(&l, shadowed, vec![drop(), returned()])), &l), 1);
        let held = stmt(StmtKind::Assign(Expr::var("x"), Expr::var("y"))).knowing(Known::Drop);
        assert_eq!(breaks(&after(block(&l, vec![jump()], vec![held, returned()])), &l), 1);
        assert_eq!(breaks(&after(block(&l, vec![jump()], vec![returned()])), &l), 1);
        let program = Name::label("found");
        assert_eq!(
            breaks(
                &after(block(
                    &program,
                    vec![stmt(StmtKind::Break(Some(program.clone())))],
                    vec![drop(), returned()]
                )),
                &program
            ),
            1
        );
    }
}
