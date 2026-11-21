//! A call of an entry of a `&mut`'s dictionary through its box,
//! `$throughBox(TAsRefStr, { asRef: [1] }).asRef(held)` (#1414): the entry of
//! what it's to, each argument it gives `1` its box's `value`, `2` a list of
//! boxes' values, `TAsRefStr.asRef(held.value)`, as `$throughBox`'s entry
//! calls it (#1470). A dictionary's entry reads no `this` (ADR 0882), so
//! called on it is called alone.

use super::*;

/// `callee(args)` folded so, `args` taken; `None` of another call.
pub(super) fn called(callee: &Expr, args: &mut Vec<Expr>) -> Option<Expr> {
    let ExprKind::Member(through, name) = &callee.kind else {
        return None;
    };
    let ExprKind::Call(helper, made) = &through.kind else {
        return None;
    };
    let ([dictionary, entries], Some("$throughBox")) = (made.as_slice(), helper.helper_name()) else {
        return None;
    };
    let ExprKind::Object(props, _) = &entries.kind else {
        return None;
    };
    let given = props.iter().find_map(|prop| match prop {
        Prop::Field(entry, given) if entry == name => Some(given),
        _ => None,
    });
    let kinds: Option<Vec<i128>> = match given {
        None => Some(Vec::new()),
        Some(given) => match &given.kind {
            ExprKind::Array(items, _) => items.iter().map(Expr::as_int).collect(),
            _ => None,
        },
    };
    let kinds = kinds?;
    let args = (std::mem::take(args).into_iter().enumerate())
        .map(|(i, arg)| match kinds.get(i) {
            Some(1) => Expr::member(arg, HELD),
            Some(2) => {
                let item = Name::declared("item".to_string());
                let read = Expr::member(Expr::var(&item), HELD);
                Expr::call(
                    Expr::member(arg, "map"),
                    vec![Expr::arrow(
                        vec![item.into()],
                        vec![StmtKind::Return(Some(read)).at(Span::NONE)],
                    )],
                )
            }
            _ => arg,
        })
        .collect();
    Some(Expr::call(Expr::member(dictionary.clone(), name.clone()), args))
}
