//! A map's entry, as tidy leaves it: the value there or put in where it's
//! missing, then used or stepped, the map's own steps, `if (!m.has(k)) {
//! m.set(k, []); } m.get(k).push(v);` (ADR 0059, amended). Split from
//! map.rs by concept, as a file is at most 1500 lines.

use crate::js::{self, Expr, Stmt, StmtKind};

/// `$orInsertWith(m, k, () => []).push(v)`, `m.entry(k).or_default().push(v)`,
/// the value there or put in that's the first its statement does: `if
/// (!m.has(k)) { m.set(k, []); } m.get(k).push(v);`, as a person writes
/// it, the helper's own steps in its order. Where the map and the key read
/// the same each time, and what makes the value is one value, which runs
/// where the helper's maker would; one given drops keeps the helper.
/// `or_insert(v)`'s `$orInsert(m, k, v)` too where `v` only reads
/// variables, which Rust evaluates whether or not it's put in: tidy's step
/// of a map's (`mir::tidy`).
pub(in crate::lower) fn inserted_entries(stmts: &mut Vec<Stmt>) {
    // What a statement does first, where it's an expression's: a call's
    // callee, a member's object, an operator's first operand.
    fn first(e: &mut Expr) -> &mut Expr {
        match e.kind {
            js::ExprKind::Call(ref callee, _) if callee.helper_name().is_some() => e,
            // The value there, `$mutGet(m, k).value`, read whole.
            js::ExprKind::Member(ref object, ref field) if field == js::HELD && got(object).is_some() => e,
            js::ExprKind::Call(..)
            | js::ExprKind::Member(..)
            | js::ExprKind::Index(..)
            | js::ExprKind::Binary(..)
            | js::ExprKind::OptionalMember(..) => {
                let (js::ExprKind::Call(inner, _)
                | js::ExprKind::Member(inner, _)
                | js::ExprKind::Index(inner, _)
                | js::ExprKind::Binary(_, inner, _)
                | js::ExprKind::OptionalMember(inner, _)) = &mut e.kind
                else {
                    unreachable!("matched")
                };
                first(inner)
            }
            _ => e,
        }
    }
    let mut i = 0;
    while i < stmts.len() {
        let span = stmts[i].span;
        // Or put in as its statement alone, `$orInsertWith(m, k, () => v);`,
        // the value there then read first by the next, `return $mutGet(m,
        // k).value;`, MIR's `*m.entry(k).or_insert_with(f)` of a value it
        // copies: `if (!m.has(k)) { m.set(k, v); } return m.get(k);` (the
        // vercel-ui SDK's port, G3).
        let put = match &stmts[i].kind {
            StmtKind::Expr(e) => put_in(e),
            _ => None,
        };
        if let Some((map, key, made)) = put
            && let Some(next) = stmts.get_mut(i + 1)
            && let Some(read) = value_of(next).map(first)
            && let js::ExprKind::Member(object, _) = &read.kind
            && let Some([m, k]) = got(object)
            && js::same_path(m, &map)
            && same_key(k, &key)
        {
            *read = Expr {
                span: read.span,
                ..Expr::call(Expr::member(map.clone(), "get"), vec![key.clone()])
            };
            stmts[i] = missing_set(map, key, made, span);
            i += 2;
            continue;
        }
        let slot = value_of(&mut stmts[i]);
        // What's done with it, `.push(v)`: one that's the statement's value
        // alone, `const current = $orInsert(..)`, a count's, stays.
        let Some(slot) = slot
            .filter(|e| !matches!(&e.kind, js::ExprKind::Call(callee, _) if callee.helper_name().is_some()))
            .map(first)
        else {
            i += 1;
            continue;
        };
        let Some((map, key, made)) = put_in(slot) else {
            i += 1;
            continue;
        };
        *slot = Expr {
            span: slot.span,
            ..Expr::call(Expr::member(map.clone(), "get"), vec![key.clone()])
        };
        stmts.insert(i, missing_set(map, key, made, span));
        i += 2;
    }
}

/// What `e`, `$orInsertWith(m, k, () => v)` or `$orInsert(m, k, v)`, puts
/// in where it's missing: the map, the key and the value, where the map and
/// the key read the same each time and the value is one value, which runs
/// where the helper's maker would; `$orInsert`'s only of variables, which
/// Rust evaluates whether or not it's put in.
fn put_in(e: &Expr) -> Option<(Expr, Expr, Expr)> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    match (callee.helper_name(), args.as_slice()) {
        (Some("$orInsertWith"), [map, key, make]) if map.reads_same() && key.reads_same() => match &make.kind {
            js::ExprKind::Arrow(params, body) if params.is_empty() => match body.as_slice() {
                [
                    Stmt {
                        kind: StmtKind::Return(Some(made)),
                        ..
                    },
                ] => Some((map.clone(), key.clone(), made.clone())),
                _ => None,
            },
            _ => None,
        },
        (Some("$orInsert"), [map, key, made]) if map.reads_same() && key.reads_same() && made.reads_only_vars() => {
            Some((map.clone(), key.clone(), made.clone()))
        }
        _ => None,
    }
}

/// Whether two keys an entry's steps are given are one: one path, or one
/// text written out, `"a"` (`counted_entries`, `inserted_entries`).
fn same_key(a: &Expr, b: &Expr) -> bool {
    js::same_path(a, b) || matches!((&a.kind, &b.kind), (js::ExprKind::Str(a), js::ExprKind::Str(b)) if a == b)
}

/// `$mutGet(m, k)`'s map and key.
fn got(e: &Expr) -> Option<&[Expr; 2]> {
    match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$mutGet") => args.as_slice().try_into().ok(),
        _ => None,
    }
}

/// The expression a statement's value is, of those `inserted_entries`
/// reads: a `const`'s, a `return`'s, an expression's, a variable's set.
fn value_of(stmt: &mut Stmt) -> Option<&mut Expr> {
    match &mut stmt.kind {
        StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) | StmtKind::Expr(e) => Some(e),
        StmtKind::Assign(target, e) if matches!(target.kind, js::ExprKind::Var(_)) => Some(e),
        _ => None,
    }
}

/// `if (!m.has(k)) { m.set(k, v); }`: the value put in where it's missing.
fn missing_set(map: Expr, key: Expr, made: Expr, span: js::Span) -> Stmt {
    let missing = Expr::unary(
        js::UnaryOp::Not,
        Expr::call(Expr::member(map.clone(), "has"), vec![key.clone()]),
    );
    let set = Expr::call(Expr::member(map, "set"), vec![key, made]);
    StmtKind::If(missing, vec![StmtKind::Expr(set).at(span)], None).at(span)
}

/// `$orInsert(m, k, 0); const h = $mutGet(m, k); h.value = f(h.value);`,
/// MIR's `*m.entry(k).or_insert(0) += 1`, the handle read nowhere else: the
/// value there or put in, then set, `const current = $orInsert(m, k, 0);
/// m.set(k, f(current));`: tidy's step of a map's (`mir::tidy`).
pub(in crate::lower) fn counted_entries(stmts: &mut Vec<Stmt>) {
    let call_of = |e: &Expr, helper: &str| match &e.kind {
        js::ExprKind::Call(callee, args) if matches!(&callee.kind, js::ExprKind::Var(f) if f.is(helper)) => {
            Some(args.clone())
        }
        _ => None,
    };
    let mut i = 0;
    while i + 2 < stmts.len() {
        if let StmtKind::Expr(inserted) = &stmts[i].kind
            && let Some(args) = call_of(inserted, "$orInsert")
            && let [map, key, _] = &args[..]
            && let StmtKind::Const(handle, got) = &stmts[i + 1].kind
            && let Some(got) = call_of(got, "$mutGet")
            && let [m, k] = &got[..]
            && js::same_path(map, m)
            && same_key(key, k)
            && let StmtKind::Assign(target, value) = &stmts[i + 2].kind
            && matches!(&target.kind, js::ExprKind::Member(of, v)
                if v == js::HELD && matches!(&of.kind, js::ExprKind::Var(h) if h == handle))
            && js::mentions_of(&stmts[i + 3..], handle) == 0
        {
            let name = (std::iter::once("current".to_string()))
                .chain((1..).map(|n| format!("current${n}")))
                .find(|n| !js::names_of(stmts).iter().any(|name| name.is(n)))
                .expect("a name");
            let name = js::Name::declared(name);
            let mut value = vec![StmtKind::Expr(value.clone()).at(js::Span::NONE)];
            js::each_expr_mut(&mut value, &mut |e| {
                if let js::ExprKind::Member(of, v) = &e.kind
                    && v == js::HELD
                    && matches!(&of.kind, js::ExprKind::Var(h) if h == handle)
                {
                    *e = Expr::var(name.clone());
                }
            });
            if js::mentions_of(&value, handle) > 0 {
                i += 1;
                continue;
            }
            let StmtKind::Expr(value) = value.pop().expect("one").kind else {
                unreachable!("made")
            };
            let set = Expr::call(Expr::member(map.clone(), "set"), vec![key.clone(), value]);
            let span = stmts[i + 2].span;
            stmts[i].kind = StmtKind::Const(name, inserted.clone());
            stmts.splice(i + 1..i + 3, [StmtKind::Expr(set).at(span)]);
        }
        i += 1;
    }
}
