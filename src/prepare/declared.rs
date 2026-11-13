//! A function a `const` binds, declared, `function deleteChild(..) { .. }`,
//! as JS writes a helper (ADRs 0882, 0972): a block's closure of
//! statements, a module's function made as a value, and one stored as a
//! property named as it. Split from prepare.rs by concept, as a file is at
//! most 1500 lines.

use crate::js;
use crate::js::{Expr, ExprKind, Stmt, StmtKind};

/// A module's `const` of a function made as a value, a thread-local's
/// `js::function::with_this` function: declared, `function onUnsuspend()
/// { this.count--; }`, as upstream writes it and as a block's is
/// (`declared_functions`, ADR 0972, amended). Its declaration's hoisting
/// changes nothing a `const` reads: no item is set again, and one read
/// before its `const` threw.
pub(super) fn declared_module_functions(items: &mut [js::Item]) {
    for item in items.iter_mut() {
        if let js::Item::Const(constant) = item
            && !constant.mutable
            && matches!(&constant.value.kind, ExprKind::Function(function) if function.name.is_empty())
            && let js::Item::Const(constant) = std::mem::replace(item, js::Item::Statements(Vec::new()))
            && let ExprKind::Function(mut function) = constant.value.kind
        {
            function.name = constant.name;
            function.export = constant.export;
            function.span = constant.span;
            *item = js::Item::Function(*function);
        }
    }
}

/// `const deleteChild = (returnFiber, child) => { .. };`, a closure of
/// statements a `let` binds: a function declared as React writes its
/// factory's, `function deleteChild(returnFiber, child) { .. }` (ADR 0882,
/// amended). One of a value alone, `const double = (x) => x * 2`, stays an
/// arrow, as a field's does. Where its declaration's hoisting changes
/// nothing: a `const`, never set again, whose name no read before it can
/// reach, as JS's `const` throws there, and the names of a block are given
/// apart; not one that reads `this` or `arguments`, which a function has
/// of its own. A function made as a value, `js::function::with_this`'s,
/// is declared so too, a value alone as well, its `this` its own as it
/// was; and one stored as a property is named as it, `update.callback =
/// function callback() { .. }` (ADR 0972).
pub(super) fn declared_functions(stmts: &mut [Stmt]) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::Assign(target, value) = &mut stmt.kind
            && let ExprKind::Member(_, property) = &target.kind
        {
            named_as(value, property);
            continue;
        }
        if let StmtKind::Const(_, value) = &stmt.kind
            && matches!(&value.kind, ExprKind::Function(function) if function.name.is_empty())
            && let StmtKind::Const(name, value) = std::mem::replace(&mut stmt.kind, StmtKind::Comments)
            && let ExprKind::Function(mut function) = value.kind
        {
            function.name = name;
            function.span = stmt.span;
            stmt.kind = StmtKind::Function(function);
            continue;
        }
        let StmtKind::Const(name, value) = &stmt.kind else {
            continue;
        };
        let (ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body)) = &value.kind else {
            continue;
        };
        // A function held as a value, an `Rc`'s or a `Box`'s of a closure,
        // stays a `const` of its arrow (ADR 0882, amended).
        if value.facts.has(js::Known::Valued) {
            continue;
        }
        let mut own = js::reads_this(body);
        js::each_expr(body, &mut |e| {
            own |= matches!(&e.kind, ExprKind::Var(v) if v == "arguments")
        });
        let value_alone = matches!(
            body.as_slice(),
            [Stmt {
                kind: StmtKind::Return(Some(_)),
                ..
            }]
        );
        if own || value_alone {
            continue;
        }
        let function = js::Function {
            name: name.clone(),
            params: params.clone(),
            body: body.clone(),
            export: false,
            is_async: matches!(value.kind, ExprKind::AsyncArrow(..)),
            noinline: false,
            span: stmt.span,
            name_span: js::Span::NONE,
            param_spans: Vec::new(),
            set_apart: false,
        };
        stmt.kind = StmtKind::Function(Box::new(function));
    }
}

/// `value`, where it's a function of no name, named `property`, as the
/// property it's stored as: where that's a name a function may have, and
/// one it reads nothing by, which its own name would be inside it.
fn named_as(value: &mut Expr, property: &str) {
    let ExprKind::Function(function) = &mut value.kind else {
        return;
    };
    if function.name.is_empty()
        && crate::names::is_identifier(property)
        && !crate::names::kept_word(property)
        && js::mentions_in(&function.body, property) == 0
    {
        function.name = property.to_owned();
    }
}
