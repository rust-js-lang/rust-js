//! A `RefCell`'s momentary borrow, held only while nothing can ask (rd218),
//! checked and not counted (ADR 1141, amended): the borrow owner's
//! decision, `lower::std_types::cell`'s, written as a call,
//! `$borrowMut(cell)`, which reads as before wherever it is, a loop's or a
//! short circuit's test too; and where it's evaluated first in its
//! statement, its check, `if (cell.borrows) $alreadyBorrowed();`, then the
//! cell, with no call where the cell is free (tidy's `borrow checks
//! first`, `Vec::remove`'s lowering).

use super::*;

impl Expr {
    /// The momentary borrow of `cell`, mutable or not: `$borrowMut(cell)`,
    /// `$borrow(cell)`, `Known::Momentary`.
    pub fn momentary_borrow(cell: Expr, mutable: bool) -> Expr {
        let call = Expr::call_helper(if mutable { "$borrowMut" } else { "$borrow" }, vec![cell]);
        Expr {
            facts: call.facts.with(Known::Momentary),
            ..call
        }
    }

    /// Of a momentary borrow, its cell and whether it's mutable.
    pub fn momentary_cell(&self) -> Option<(&Expr, bool)> {
        if !self.facts.has(Known::Momentary) {
            return None;
        }
        let ExprKind::Call(callee, given) = &self.kind else {
            return None;
        };
        match (callee.helper_name(), given.as_slice()) {
            (Some("$borrowMut"), [cell]) => Some((cell, true)),
            (Some("$borrow"), [cell]) => Some((cell, false)),
            _ => None,
        }
    }
}

impl Stmt {
    /// A momentary borrow's check, where it's evaluated first: `if
    /// (cell.borrows) $alreadyBorrowed();` of a mutable one, `if
    /// (cell.borrows < 0) $alreadyMutablyBorrowed();` of a shared one,
    /// std's panics.
    pub fn borrow_check(cell: Expr, mutable: bool, span: Span) -> Stmt {
        let borrows = Expr::member(cell, "borrows");
        let (taken, refused) = match mutable {
            true => (borrows, "$alreadyBorrowed"),
            false => (Expr::bin(Op::Lt, borrows, Expr::int(0)), "$alreadyMutablyBorrowed"),
        };
        let refused = StmtKind::Expr(Expr::call_helper(refused, Vec::new())).at(span);
        StmtKind::If(taken, vec![refused], None).at(span)
    }

    /// Each momentary borrow evaluated first in this statement, before
    /// anything else that does something, written as its check before it
    /// and its cell in its place: the checks, in the order they run. A test
    /// a loop evaluates again, or a branch of a short circuit or a
    /// conditional, keeps its borrow, `$borrow(cell)`.
    pub fn borrow_checks_first(&mut self) -> Vec<Stmt> {
        let span = self.span;
        let first = match &mut self.kind {
            StmtKind::Expr(e)
            | StmtKind::Const(_, e)
            | StmtKind::Let(_, Some(e))
            | StmtKind::Return(Some(e))
            | StmtKind::Throw(e)
            | StmtKind::If(e, ..)
            | StmtKind::Destructure { value: e, .. } => e,
            StmtKind::Assign(target, value) if !target.has_effects() => value,
            _ => return Vec::new(),
        };
        let mut checks = Vec::new();
        while let Taken::Cell(cell, mutable) = taken_first(first) {
            checks.push(Stmt::borrow_check(cell, mutable, span));
        }
        checks
    }
}

/// Each statement of `stmts` with its momentary borrows evaluated first
/// written as their checks before it (`Stmt::borrow_checks_first`).
/// A `const` that held the borrow, `const borrowMut = $borrowMut(nodes)`,
/// its cell once its check is first, is the cell, read by its name where
/// nothing after sets either or declares the cell's again. A variable that
/// held it, its own cell, a guard a `let` holds named as the parameter it
/// borrows, `state = $borrowMut(state)`, is the check alone (#1815).
pub fn borrow_checks_first(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let checks = stmts[i].borrow_checks_first();
        let n = checks.len();
        let itself = matches!(&stmts[i].kind, StmtKind::Assign(
            Expr { kind: ExprKind::Var(to), .. },
            Expr { kind: ExprKind::Var(cell), .. },
        ) if n > 0 && to == cell);
        if itself {
            stmts.splice(i..=i, checks);
            i += n;
            continue;
        }
        let alias = match &stmts[i].kind {
            StmtKind::Const(
                name,
                Expr {
                    kind: ExprKind::Var(cell),
                    ..
                },
            ) if n > 0 => Some((name.clone(), cell.clone())),
            _ => None,
        };
        stmts.splice(i..i, checks);
        i += n;
        if let Some((name, cell)) = alias {
            let rest = &stmts[i + 1..];
            let mut read = rest.to_vec();
            each_expr_mut(&mut read, &mut |e| {
                if matches!(&e.kind, ExprKind::Var(v) if *v == name) {
                    e.kind = ExprKind::Var(cell.clone());
                }
            });
            // Nothing after declares either again, nor sets them, and each
            // read of the cell there is of it, none a declaration spelled
            // as it takes, as JS's scopes say.
            if !declares(rest, &cell)
                && !declares(rest, &name)
                && !Writes::sets(rest, &cell)
                && !Writes::sets(rest, &name)
                && super::scopes::not_read_as_themselves(&read, &[&cell]).is_empty()
            {
                stmts.splice(i + 1.., read);
                stmts.remove(i);
                continue;
            }
        }
        i += 1;
    }
}

/// What evaluating `e` meets first: a momentary borrow of a cell read the
/// same twice, which is taken, its cell in its place; something else that
/// does something, where the walk stops; or nothing that does.
enum Taken {
    Cell(Expr, bool),
    Stopped,
    Nothing,
}

fn taken_first(e: &mut Expr) -> Taken {
    if let Some((cell, mutable)) = e.momentary_cell()
        && cell.reads_same()
        && !cell.has_effects()
    {
        let cell = cell.clone();
        *e = cell.clone();
        return Taken::Cell(cell, mutable);
    }
    // Its parts in the order JS evaluates them, those it always does; and
    // whether what it does then, or the parts it may not evaluate, stop
    // the walk.
    let (parts, stops): (Vec<&mut Expr>, bool) = match &mut e.kind {
        ExprKind::Member(object, _) => (vec![&mut **object], false),
        ExprKind::Index(object, index) => (vec![&mut **object, &mut **index], false),
        ExprKind::Call(callee, given) => (std::iter::once(&mut **callee).chain(given.iter_mut()).collect(), true),
        ExprKind::Binary(Op::And | Op::Or | Op::Coalesce, left, _) => (vec![&mut **left], true),
        ExprKind::Binary(_, left, right) => (vec![&mut **left, &mut **right], false),
        ExprKind::Unary(_, operand) => (vec![&mut **operand], false),
        ExprKind::Cond(test, ..) | ExprKind::SomeWhere(test, _) => (vec![&mut **test], true),
        ExprKind::Array(items) => (items.iter_mut().collect(), false),
        _ => {
            return if e.has_effects() {
                Taken::Stopped
            } else {
                Taken::Nothing
            };
        }
    };
    for part in parts {
        match taken_first(part) {
            Taken::Nothing => {}
            done => return done,
        }
    }
    match stops || e.has_effects() {
        true => Taken::Stopped,
        false => Taken::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `const borrowMut = $borrowMut(nodes); if (c) { const nodes = 1;
    // f(borrowMut, nodes); }`: the alias is the cell where nothing after
    // takes the cell's word, as `f(nodes)` there would read the block's.
    #[test]
    fn an_alias_is_its_cell_but_where_a_block_declares_the_cells_word() {
        let checked = |shadowed: bool| {
            let (cell, alias) = (Name::declared("nodes".into()), Name::declared("borrowMut".into()));
            let inner = Name::declared(if shadowed { "nodes" } else { "count" }.into());
            let call = Expr::call(Expr::var("f"), vec![Expr::var(&alias), Expr::var(&inner)]);
            let block = vec![
                StmtKind::Const(inner.clone(), Expr::int(1)).at(Span::NONE),
                StmtKind::Expr(call).at(Span::NONE),
            ];
            let mut stmts = vec![
                StmtKind::Const(alias.clone(), Expr::momentary_borrow(Expr::var(&cell), true)).at(Span::NONE),
                StmtKind::If(Expr::var("c"), block, None).at(Span::NONE),
            ];
            borrow_checks_first(&mut stmts);
            (stmts.iter()).any(|s| matches!(&s.kind, StmtKind::Const(name, _) if *name == alias))
        };
        assert!(!checked(false));
        assert!(checked(true));
    }
}
