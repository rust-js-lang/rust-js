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
}

fn block(body: &mut [Stmt]) {
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
