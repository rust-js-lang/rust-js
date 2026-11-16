//! What a `try` holds where it's a closure called where it's made,
//! `catch_unwind(|| ..)`'s (ADR 0660): its body in its place (ADR 0706),
//! where its statement is (rd630), and a variable its value is given to
//! set where the value is made (rd534).

use super::*;

/// Where a `try`'s statement of a value is, made of `value`, where the
/// value is written, and of what it's `made_of`, a `match`'s head through
/// its `Ok` arm: from the value's start through the end of either, so the
/// comments above the arm, inside it, are its (rd630). Either alone where
/// the other has no place, `made_of` where the value has none.
pub fn tried_extent(value: Span, made_of: Span) -> Span {
    match (value.is_none(), made_of.is_none()) {
        (true, _) => made_of,
        (false, true) => value,
        (false, false) => Span {
            lo: value.lo,
            hi: value.hi.max(made_of.hi),
        },
    }
}

/// What a `try` holds, `tried`, a block of its own, with a closure of
/// nothing called where it's made, `catch_unwind(|| ..)`'s (ADR 0660),
/// as its body: `return (() => { .. })();` its body, each `return` of it
/// the function's, as returning what it returns is, where it never ends
/// without one, as a closure of a value doesn't; and `(() => { .. })();`
/// first, its body where it never returns, or returns only at its end,
/// whose value it's given to nothing: `return step(n);` is `step(n);`
/// (ADR 0706, amended). Not of an async closure, whose value is a promise.
pub fn applied_tried(mut tried: Vec<Stmt>) -> Vec<Stmt> {
    let body_of = |value: &Expr| called_body(value).map(<[Stmt]>::to_vec);
    match tried.first().map(|s| &s.kind) {
        Some(StmtKind::Return(Some(value))) if tried.len() == 1 => {
            match body_of(value).filter(|body| body.last().is_some_and(ends)) {
                Some(body) => body,
                None => tried,
            }
        }
        // What its body declares is named apart from the function's other
        // names, as lowering names each, until reclaimed (ADR 0352).
        Some(StmtKind::Expr(value)) => match body_of(value) {
            Some(body) if !returns(&body).0 => {
                tried.splice(0..1, body);
                tried
            }
            Some(mut body)
                if body
                    .split_last()
                    .is_some_and(|(last, before)| !returns(before).0 && matches!(last.kind, StmtKind::Return(_))) =>
            {
                let last = body.pop().expect("a last statement");
                if let StmtKind::Return(Some(value)) = last.kind
                    && value.has_effects()
                {
                    body.push(StmtKind::Expr(value).at(last.span));
                }
                tried.splice(0..1, body);
                tried
            }
            _ => tried,
        },
        // Its value the `try` keeps, `const value = (() => { ..; return v;
        // })();`, of a body that returns only at its end: its body, then
        // `const value = v;`, as what follows reads it, what the body
        // declares named apart from the rest as above (ADR 0660, amended).
        // So is a variable set, `result = (() => { ..; return v; })();`,
        // `result = v;`, React's createChildReconciler's (rd551).
        Some(
            StmtKind::Const(_, value)
            | StmtKind::Assign(
                Expr {
                    kind: ExprKind::Var(_), ..
                },
                value,
            ),
        ) => match body_of(value) {
            Some(mut body)
                if body.split_last().is_some_and(|(last, before)| {
                    !returns(before).0 && matches!(last.kind, StmtKind::Return(Some(_)))
                }) =>
            {
                let last = body.pop().expect("a last statement");
                let StmtKind::Return(Some(value)) = last.kind else {
                    unreachable!("matched above")
                };
                let kept = match &tried[0].kind {
                    StmtKind::Const(name, _) => StmtKind::Const(name.clone(), value),
                    StmtKind::Assign(to, _) => StmtKind::Assign(to.clone(), value),
                    _ => unreachable!("matched above"),
                };
                // Where the `return` was, after what's said before it;
                // through what the statement it's of is of, a `match`'s `Ok`
                // arm, `href = match catch_unwind(|| parse(s)) { Ok(h) => h,
                // .. }`'s, whose comments are its (rd630).
                let span = tried_extent(last.span, tried[0].span);
                body.push(kept.at(span));
                tried.splice(0..1, body);
                tried
            }
            // A body that returns early, a guard's `if (n === 0) { return
            // 0; }`: each `return v;` sets the value, `found = 0;`, what
            // follows the guard its `else`, as a person writes it, where
            // each `return` ends what runs of it (rd60).
            Some(body) => {
                let (declared, to) = match &tried[0].kind {
                    StmtKind::Const(name, _) => (Some(name.clone()), Expr::var(name)),
                    StmtKind::Assign(to, _) => (None, to.clone()),
                    _ => unreachable!("matched above"),
                };
                // Not where the body declares the variable's name, which
                // its blocks would set in its place.
                let shadowed = matches!(&to.kind, ExprKind::Var(name) if declares(&body, name));
                let set = |value: &Expr, at: Span| Some(vec![StmtKind::Assign(to.clone(), value.clone()).at(at)]);
                let Some(set) = returns_replaced(&body, &set).filter(|_| !shadowed) else {
                    return tried;
                };
                let span = tried[0].span;
                let declared = declared.map(|name| StmtKind::Let(name, None).at(span));
                tried.splice(0..1, declared.into_iter().chain(set));
                tried
            }
            _ => tried,
        },
        _ => tried,
    }
}

/// Of `body`, each `try` whose closure's value `applied_tried` gave a
/// variable declared just before it, `let didFatal; try { const didFatal$1 =
/// throwException(..); ..; didFatal = didFatal$1; } catch ..`, that sets
/// the variable where the value is made, `didFatal = throwException(..);
/// ..`, as React's throwAndUnwindWorkLoop writes it (rd534, ADR 0706
/// amended). Only where that's the same program: what follows the value
/// in the `try` never names the variable, so reads none of it set early,
/// and reads the value in no closure, which may run once the variable
/// changes; and where what follows may throw, the variable set early is
/// never seen: the `catch` never names it and never goes on, leaving the
/// block the variable is of, or sets it before anything names it; and no
/// function the block declares, which runs where it's called, names it.
pub fn tried_set_where_made(body: &mut [Stmt]) {
    for i in 1..body.len() {
        let declared = body[..i]
            .iter()
            .rev()
            .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
            .count();
        let lets = &body[i - declared.max(1)..i];
        let declared: Vec<String> = (lets.iter())
            .filter_map(|s| match &s.kind {
                StmtKind::Let(name, _) => Some(name.to_string()),
                _ => None,
            })
            .collect();
        // A value its arm binds is a copy too, `const found = found$1;
        // done = found;`, set where it's made in turn.
        while let StmtKind::TryCatch(tried, binding, caught) = &body[i].kind {
            let Some((made, set, variable)) =
                set_where_made(tried, binding.as_ref().map(|n| n.word()), caught, |name| {
                    declared.iter().any(|n| n == name)
                })
            else {
                break;
            };
            if body
                .iter()
                .any(|s| matches!(&s.kind, StmtKind::Function(f) if mentions_in(&f.body, &variable) > 0))
            {
                break;
            }
            set_at(&mut body[i], made, set, &variable);
        }
    }
}

/// `try`'s `v = value;` at `set` gone, `variable` set where the `const` at
/// `made` makes the value.
fn set_at(try_catch: &mut Stmt, made: usize, set: usize, variable: &str) {
    let StmtKind::TryCatch(tried, ..) = &mut try_catch.kind else {
        unreachable!("a try")
    };
    // A comment before the value, `// What it found.` above the closure's
    // `did_fatal`, is where it was, after what follows.
    let span = tried[set].span;
    let StmtKind::Assign(to, _) = std::mem::replace(&mut tried[set].kind, StmtKind::Comments) else {
        unreachable!("found by set_where_made")
    };
    if span.is_none() {
        tried.remove(set);
    }
    let StmtKind::Const(value_name, value) = &tried[made].kind else {
        unreachable!("found by set_where_made")
    };
    let value_name = value_name.clone();
    // Its reads the variable's, the one it's set to now.
    let into = match &to.kind {
        ExprKind::Var(into) => Some(into.clone()),
        _ => None,
    };
    tried[made].kind = StmtKind::Assign(to, value.clone());
    match into {
        Some(into) => merged(&mut tried[made + 1..set], value_name.word(), &into),
        None => renamed(&mut tried[made + 1..set], value_name.word(), variable),
    }
}

/// Of a `try`'s `tried` and its `catch`, the `const` of the value made, the
/// variable's `v = value;` and the variable, a `declared` one, where setting
/// it where the value is made is the same program (`tried_set_where_made`).
fn set_where_made(
    tried: &[Stmt],
    binding: Option<&str>,
    caught: &[Stmt],
    declared: impl Fn(&str) -> bool,
) -> Option<(usize, usize, String)> {
    tried.iter().enumerate().find_map(|(set, stmt)| {
        let StmtKind::Assign(to, from) = &stmt.kind else {
            return None;
        };
        let (ExprKind::Var(variable), ExprKind::Var(value)) = (&to.kind, &from.kind) else {
            return None;
        };
        let made = tried[..set]
            .iter()
            .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == value))?;
        let between = &tried[made + 1..set];
        // The `catch` leaves the variable's block, naming it nowhere, or
        // sets it before it names it.
        let unseen = |caught: &[Stmt]| {
            let leaves = caught.last().is_some_and(ends) && mentions_of(caught, variable) == 0;
            let first_set = caught.iter().position(|s| {
                matches!(&s.kind, StmtKind::Assign(t, v)
                    if matches!(&t.kind, ExprKind::Var(n) if n == variable) && !v.mentions_var(variable.word()))
            });
            leaves || first_set.is_some_and(|first| mentions_of(&caught[..first], variable) == 0)
        };
        let same = declared(variable.word())
            && mentions_of(&tried[..set], variable) == 0
            && !declares(between, variable)
            && mentions_of(&tried[set + 1..], value) == 0
            && !read_in_a_closure(between, value.word())
            && (between.iter().all(Stmt::cannot_throw) || binding != Some(variable.word()) && unseen(caught));
        same.then(|| (made, set, variable.to_string()))
    })
}

/// Whether a closure or function in `stmts` names `name`.
fn read_in_a_closure(stmts: &[Stmt], name: &str) -> bool {
    let mut read = false;
    each_expr(stmts, &mut |e| match &e.kind {
        ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => read |= mentions_in(body, name) > 0,
        ExprKind::Function(function) => read |= mentions_in(&function.body, name) > 0,
        _ => {}
    });
    each_list(stmts, &mut |list| {
        read |= (list.iter()).any(|s| matches!(&s.kind, StmtKind::Function(f) if mentions_in(&f.body, name) > 0));
    });
    read
}
