//! Readability preparation on the completed JS tree. No rustc types or APIs.
//! Nothing moves: what JSX holds stays where it's written, a list's callback
//! of several statements and a handler's too, as a person writes them, and
//! oxfmt lays them out where they are (ADR 0218). The printer only lays out
//! the result.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, JsxTag, Prop, Stmt, StmtKind};

pub fn module(module: &mut js::Module) {
    let methods = module.namespaces.iter_mut().flat_map(|n| n.methods.iter_mut());
    for function in module.functions.iter_mut().chain(methods) {
        block(&mut function.body);
        shadows(&mut function.body);
        constants(&mut function.body);
    }
    for constant in &mut module.consts {
        expr(&mut constant.value);
    }
    block(&mut module.statements);
}

fn block(body: &mut Vec<Stmt>) {
    try_catches(body);
    for stmt in body.iter_mut() {
        if let Some(coalesced) = coalescing(stmt) {
            stmt.kind = coalesced;
        }
        // `else {}`, of nothing but Rust's comments, is no `else`.
        if let StmtKind::If(_, _, els) = &mut stmt.kind
            && els.as_ref().is_some_and(Vec::is_empty)
        {
            *els = None;
        }
    }
    for stmt in body {
        match &mut stmt.kind {
            StmtKind::Const(_, e)
            | StmtKind::Expr(e)
            | StmtKind::Throw(e)
            | StmtKind::Return(Some(e))
            | StmtKind::Let(_, Some(e))
            | StmtKind::Destructure { value: e, .. } => expr(e),
            StmtKind::Assign(a, b) => {
                expr(a);
                expr(b);
            }
            StmtKind::If(e, a, b) => {
                expr(e);
                block(a);
                if let Some(b) = b {
                    block(b);
                }
            }
            StmtKind::While { cond, body, .. } => {
                expr(cond);
                block(body);
            }
            StmtKind::Labeled(_, body) => block(body),
            StmtKind::Try(body, finally) | StmtKind::TryCatch(body, _, finally) => {
                block(body);
                block(finally);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                expr(iterable);
                block(body);
            }
            StmtKind::For { start, test, body, .. } => {
                expr(start);
                expr(test);
                block(body);
            }
            _ => {}
        }
    }
}

/// `const match = $try(() => f()); if (match.TAG === "Ok") { x = match._0; }
/// else { .. }`, a `match` of what a JS call threw whose `Ok` only keeps
/// the value and whose `Err` reads no error, is `try { x = f(); } catch {
/// .. }`, as JS writes it (ADR 0035): only the call is in the `try`, as only
/// it is in `$try`.
fn try_catches(body: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < body.len() {
        if let Some(caught) = try_catch(&body[i], &body[i + 1], &body[i + 2..]) {
            let span = body[i].span;
            body.splice(i..i + 2, [caught.at(span)]);
        }
        i += 1;
    }
}

fn try_catch(made: &Stmt, tested: &Stmt, rest: &[Stmt]) -> Option<StmtKind> {
    let StmtKind::Const(result, value) = &made.kind else {
        return None;
    };
    let ExprKind::Call(callee, args) = &value.kind else {
        return None;
    };
    let (ExprKind::Var(helper), [thunk]) = (&callee.kind, args.as_slice()) else {
        return None;
    };
    let ExprKind::Arrow(params, thunk) = &thunk.kind else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::Return(Some(call)),
            ..
        },
    ] = thunk.as_slice()
    else {
        return None;
    };
    let StmtKind::If(test, kept, Some(failed)) = &tested.kind else {
        return None;
    };
    let ExprKind::Binary(js::Op::Eq, tag, ok) = &test.kind else {
        return None;
    };
    let of_result = |e: &Expr, name: &str| {
        matches!(&e.kind, ExprKind::Member(of, field)
        if field == name && matches!(&of.kind, ExprKind::Var(v) if v == result))
    };
    if helper != "$try"
        || !params.is_empty()
        || !of_result(tag, "TAG")
        || !matches!(&ok.kind, ExprKind::Str(s) if s == "Ok")
        || js::mentions_in(failed, result) > 0
        || js::mentions_in(rest, result) > 0
    {
        return None;
    }
    // The `Ok`'s value kept, as it is: nothing else, which could throw.
    let [Stmt { kind: kept, span }] = kept.as_slice() else {
        return None;
    };
    let body = match kept {
        StmtKind::Return(Some(e)) if of_result(e, "_0") => StmtKind::Return(Some(call.clone())),
        StmtKind::Assign(to, e) if of_result(e, "_0") && matches!(to.kind, ExprKind::Var(_)) => {
            StmtKind::Assign(to.clone(), call.clone())
        }
        _ => return None,
    };
    Some(StmtKind::TryCatch(vec![body.at(*span)], None, failed.clone()))
}

/// `if (x == null) { x = e; }` is `x = x ?? e`, printed `x ??= e`: the
/// same test, and `e` made only where it's none.
fn coalescing(stmt: &Stmt) -> Option<StmtKind> {
    let StmtKind::If(test, then, None) = &stmt.kind else {
        return None;
    };
    let ExprKind::Binary(js::Op::LooseEq, tested, null) = &test.kind else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::Assign(target, value),
            ..
        },
    ] = then.as_slice()
    else {
        return None;
    };
    if !matches!(null.kind, ExprKind::Null) || !js::same_path(tested, target) {
        return None;
    }
    Some(StmtKind::Assign(
        target.clone(),
        Expr::bin(js::Op::Coalesce, target.clone(), value.clone()),
    ))
}

/// `let list = [];` that nothing sets again, only changes in place, is
/// `const list = [];`, as JS writes it (ADR 0278). A variable a `&mut`'s
/// handle sets is set.
fn constants(body: &mut Vec<Stmt>) {
    let mut set = HashSet::new();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind {
                set.extend(root(target));
            }
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind {
            set.extend(root(place));
        }
    });
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Let(name, Some(value)) = &stmt.kind
                && !set.contains(name)
            {
                stmt.kind = StmtKind::Const(name.clone(), value.clone());
            }
        }
    });
}

/// The variable a place is, `x`: `x.a = 1` sets none.
fn root(place: &Expr) -> Option<String> {
    match &place.kind {
        ExprKind::Var(name) => Some(name.clone()),
        _ => None,
    }
}

/// `const n$1 = n;`, Rust's `let n = n;`, a variable shadowed by its own
/// value, is `n` itself: `<Type />` for `let Type = from_unknown(Type)`,
/// `content` for `let content = content.clone()`. Only where `n` holds
/// that value wherever `n$1` is read: nothing sets `n` after it, nor in a
/// closure, nor where a loop runs it again.
fn shadows(body: &mut Vec<Stmt>) {
    while let Some((alias, of)) = shadow(body) {
        js::each_expr_mut(body, &mut |e| {
            if matches!(&e.kind, ExprKind::Var(name) if *name == alias) {
                e.kind = ExprKind::Var(of.clone());
            }
        });
        js::each_block_mut(body, &mut |stmts| {
            stmts.retain(|s| {
                !matches!(&s.kind, StmtKind::Const(name, value)
                if *name == alias && matches!(&value.kind, ExprKind::Var(v) if *v == of))
            });
        });
    }
}

/// A `const n$1 = n;` that `shadows` makes `n`, and `n`.
fn shadow(body: &[Stmt]) -> Option<(String, String)> {
    // What closures set, and what their parameters are named, which a
    // name read in them would mean instead.
    let mut in_closures = HashSet::new();
    let mut copy = body.to_vec();
    js::each_expr_mut(&mut copy, &mut |e| {
        if let ExprKind::Arrow(params, stmts) | ExprKind::AsyncArrow(params, stmts) = &mut e.kind {
            in_closures.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
            js::statement_lists(stmts, &mut |list| {
                in_closures.extend(list.iter().filter_map(|s| match &s.kind {
                    StmtKind::Assign(target, _) => match &target.kind {
                        ExprKind::Var(name) => Some(name.clone()),
                        _ => None,
                    },
                    _ => None,
                }));
            });
        }
    });
    let mut walk = Walk::default();
    walk.stmts(body, false);
    walk.shadows.into_iter().find_map(|(at, looped, alias, of)| {
        let sets: Vec<usize> = walk
            .sets
            .iter()
            .filter(|(_, name)| *name == of)
            .map(|&(set, _)| set)
            .collect();
        let kept = !in_closures.contains(&of) && (sets.is_empty() || !looped && sets.iter().all(|&set| set < at));
        kept.then_some((alias, of))
    })
}

/// The function's statements in order, outside its closures: each
/// `const n$1 = n;`, where, and whether in a loop, and each variable set.
#[derive(Default)]
struct Walk {
    at: usize,
    shadows: Vec<(usize, bool, String, String)>,
    sets: Vec<(usize, String)>,
}

impl Walk {
    fn stmts(&mut self, stmts: &[Stmt], looped: bool) {
        for stmt in stmts {
            self.at += 1;
            match &stmt.kind {
                StmtKind::Const(alias, value) => {
                    if let ExprKind::Var(of) = &value.kind
                        && alias
                            .strip_prefix(of.as_str())
                            .and_then(|rest| rest.strip_prefix('$'))
                            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                    {
                        self.shadows.push((self.at, looped, alias.clone(), of.clone()));
                    }
                }
                StmtKind::Assign(target, _) => {
                    if let ExprKind::Var(name) = &target.kind {
                        self.sets.push((self.at, name.clone()));
                    }
                }
                StmtKind::If(_, a, b) => {
                    self.stmts(a, looped);
                    if let Some(b) = b {
                        self.stmts(b, looped);
                    }
                }
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } => self.stmts(body, true),
                StmtKind::For { name, body, .. } => {
                    self.sets.push((self.at, name.clone()));
                    self.stmts(body, true);
                }
                StmtKind::Labeled(_, body) => self.stmts(body, looped),
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    self.stmts(a, looped);
                    self.stmts(b, looped);
                }
                StmtKind::Let(..)
                | StmtKind::Destructure { .. }
                | StmtKind::Expr(_)
                | StmtKind::Break(_)
                | StmtKind::Continue(_)
                | StmtKind::Return(_)
                | StmtKind::Throw(_) => {}
            }
        }
    }
}

fn expr(e: &mut Expr) {
    match &mut e.kind {
        ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => block(body),
        ExprKind::Jsx(jsx) => {
            if let JsxTag::Component(e) = &mut jsx.tag {
                expr(e);
            }
            for prop in &mut jsx.props {
                match prop {
                    Prop::Field(name, value) => {
                        let handler = name
                            .strip_prefix("on")
                            .is_some_and(|s| s.starts_with(|c: char| c.is_ascii_uppercase()));
                        // React ignores handler return values: keep single-call
                        // event handlers as concise arrow expressions (ADR 0040).
                        if handler
                            && let ExprKind::Arrow(_, body) = &mut value.kind
                            && let [
                                Stmt {
                                    kind: StmtKind::Expr(e),
                                    span,
                                },
                            ] = body.as_slice()
                        {
                            *body = vec![StmtKind::Return(Some(e.clone())).at(*span)];
                        }
                        expr(value);
                    }
                    Prop::Getter(_, value) | Prop::Spread(value) => expr(value),
                }
            }
            for child in &mut jsx.children {
                expr(child);
            }
        }
        ExprKind::Member(a, _)
        | ExprKind::OptionalMember(a, _)
        | ExprKind::Unary(_, a)
        | ExprKind::Await(a)
        | ExprKind::Spread(a) => expr(a),
        ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) => {
            expr(a);
            expr(b);
        }
        ExprKind::Cond(a, b, c) => {
            expr(a);
            expr(b);
            expr(c);
        }
        ExprKind::Call(f, args) | ExprKind::New(f, args) => {
            expr(f);
            args.iter_mut().for_each(expr);
        }
        ExprKind::Array(items) => items.iter_mut().for_each(expr),
        ExprKind::Object(props) => {
            // A field that's `undefined`, a `None` say, is no key, `{ code }`,
            // as JS leaves it out: what reads it reads `undefined` either way
            // (ADR 0280). Not after a spread, whose field it would then be.
            if !props.iter().any(|p| matches!(p, Prop::Spread(_))) {
                props.retain(|p| !matches!(p, Prop::Field(_, value) if matches!(value.kind, ExprKind::Undefined)));
            }
            for prop in props {
                let (Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value)) = prop;
                expr(value);
            }
        }
        _ => {}
    }
}
