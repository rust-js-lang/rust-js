//! A trait of classes' defaults, each class's prototype's method beside
//! its own (rd283, ADR 1035).

use crate::js::{self, Expr, ExprKind, Pattern, Prop, Stmt, StmtKind};

/// Each default a class takes of a trait of classes, of its impl's
/// dictionary lowered, `body`, which returns its object: a function of
/// each default's arrow, its prototype's method, `this` its `self`; the
/// impl's own methods are its prototype's already (rd283, ADR 1035).
pub(in crate::lower) fn prototype_defaults(mut body: Vec<Stmt>) -> Vec<js::Function> {
    let Some(Stmt {
        kind: StmtKind::Return(Some(Expr {
            kind: ExprKind::Object(props, _),
            ..
        })),
        ..
    }) = body.pop()
    else {
        unreachable!("a dictionary given nothing returns its object")
    };
    let mut defaults = Vec::new();
    for prop in props {
        let Prop::Field(name, value) = prop else { continue };
        let (is_async, span) = (matches!(value.kind, ExprKind::AsyncArrow(..)), value.span);
        let (ExprKind::Arrow(mut params, mut body) | ExprKind::AsyncArrow(mut params, mut body)) = value.kind else {
            continue;
        };
        if let Some(Pattern::Name(receiver)) = (!params.is_empty()).then(|| params.remove(0))
            && js::read_as_this(&mut body, &receiver)
        {
            body.insert(0, StmtKind::Const(receiver, Expr::this()).at(js::Span::NONE));
        }
        defaults.push(js::Function {
            name,
            params,
            body,
            export: false,
            is_async,
            noinline: false,
            span,
            name_span: js::Span::NONE,
            param_spans: Vec::new(),
            set_apart: false,
        });
    }
    defaults
}

/// `function`, a default the class `owner` of a module of `namespaces`
/// takes, its prototype's method beside its own; or, where a class of the
/// module took one alike before, that class's function, assigned to each
/// (rd283, ADR 1035). `Err` of one named as a method the class has.
pub(in crate::lower) fn take_default(
    namespaces: &mut [js::Namespace],
    owner: &str,
    function: js::Function,
) -> Result<(), String> {
    let Some(at) = namespaces.iter().position(|n| n.name == owner) else {
        return Ok(());
    };
    if namespaces[at].methods.iter().any(|f| f.name == function.name) {
        return Err(format!(
            "a class's method named as a method its trait gives it, `{}`, which one prototype holds one of",
            function.name
        ));
    }
    let alike = namespaces.iter_mut().find(|n| {
        n.name != owner
            && n.class
                .as_ref()
                .is_some_and(|class| class.prototype.contains(&function.name))
            && n.methods
                .iter()
                .any(|f| f.name == function.name && f.params == function.params && f.body == function.body)
    });
    if let Some(first) = alike
        && let Some(class) = &mut first.class
    {
        match class.shared.iter_mut().find(|(method, _)| *method == function.name) {
            Some((_, others)) => others.push(owner.to_owned()),
            None => class.shared.push((function.name.clone(), vec![owner.to_owned()])),
        }
        return Ok(());
    }
    let namespace = &mut namespaces[at];
    if let Some(class) = &mut namespace.class {
        class.prototype.push(function.name.clone());
    }
    namespace.methods.push(function);
    Ok(())
}
