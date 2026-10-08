//! Readability preparation on the completed JS tree. No rustc types or APIs.
//! Nothing moves: what JSX holds stays where it's written, a list's callback
//! of several statements and a handler's too, as a person writes them, and
//! oxfmt lays them out where they are (ADR 0218). The printer only lays out
//! the result.

use crate::js::{self, Expr, ExprKind, JsxTag, Prop, Stmt, StmtKind};

pub fn module(module: &mut js::Module) {
    let methods = module.namespaces.iter_mut().flat_map(|n| n.methods.iter_mut());
    for function in module.functions.iter_mut().chain(methods) {
        block(&mut function.body);
    }
    for constant in &mut module.consts {
        expr(&mut constant.value);
    }
    block(&mut module.statements);
}

fn block(body: &mut Vec<Stmt>) {
    try_catches(body);
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
            for prop in props {
                let (Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Spread(value)) = prop;
                expr(value);
            }
        }
        _ => {}
    }
}
