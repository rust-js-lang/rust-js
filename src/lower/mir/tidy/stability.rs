//! Stability of expression reads across calls, consumed by tidy movement.
use super::values::field_path;
use crate::collections::IndexSet;
use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind, UnaryOp};

/// Whether `e` is a read of plain Rust data, which nothing changes: a field
/// however deep of a variable of plain data, or one of a plain variable's
/// of a struct not all plain, `q.title` beside a `Cell` (`Found::plain`).
pub(super) fn plain_field(e: &Expr, plain: &IndexSet<String>) -> bool {
    let mut root = e;
    while let ExprKind::Member(of, _) = &root.kind {
        root = of;
    }
    matches!(e.kind, ExprKind::Member(..))
        && (matches!(&root.kind, ExprKind::Var(v) if plain.contains(v.as_str()))
            || field_path(e).is_some_and(|path| plain.contains(&path)))
}

/// Whether reading `e` after a call reads what it did before: a constant, a
/// variable no closure sets, `set`'s, which a call can't then; an import,
/// `<Seo>`'s tag, a module's export being never set again, a `static mut`
/// its `{ value }`; a field of a plain value, `meta.tag`, which no call can
/// change, as MIR's lowering reads it where it's read, or a builtin's
/// function, `Object.fromEntries`; an element of such, a closure made, and a
/// test or a choice of such, `selected ? "page" : undefined`.
pub(super) fn read_past_calls(e: &Expr, set: &IndexSet<String>, plain: &IndexSet<String>) -> bool {
    let past = |e: &Expr| read_past_calls(e, set, plain);
    let mut root = e;
    while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {
        root = of;
    }
    let plain_root = matches!(&root.kind, ExprKind::Var(v) if plain.contains(v.as_str()) && !set.contains(v.as_str()));
    match &e.kind {
        ExprKind::Var(v) => !set.contains(v.as_str()),
        // An item of a plain value, `pair[0]`, as a field of one is.
        ExprKind::Index(_, index) => plain_root && past(index),
        ExprKind::Symbol(_) => true,
        ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,
        ExprKind::Member(of, _) => {
            // Of a variable no closure sets, a plain field of it, `a.id` of
            // an `&A` with a `Cell` beside it, too (#1045).
            plain_root
                || (plain_field(e, plain) && matches!(&root.kind, ExprKind::Var(v) if !set.contains(v.as_str())))
                // One of JS's own globals, `Math.max`, as lowering named it
                // (`Known::BuiltIn`), never a name the crate's may share
                // (#1674).
                || matches!(&of.kind, ExprKind::Var(v) if !set.contains(v.as_str()))
                    && (of.facts.has(js::Known::BuiltIn) || of.facts.has(js::Known::Lasting))
        }
        ExprKind::Jsx(_) => inert_element(e, set),
        // The items of an array nothing changes, `...first`; not a plain
        // value's, an iterator's, a function's `inspect(see)`, which
        // spreading steps, running what it calls (#1015).
        ExprKind::Spread(a) => matches!(&a.kind, ExprKind::Var(v)
            if !set.contains(v.as_str()) && plain.contains(&format!("...{v}"))),
        ExprKind::Unary(UnaryOp::Not, a) => past(a),
        ExprKind::Binary(
            Op::Eq
            | Op::Ne
            | Op::LooseEq
            | Op::LooseNe
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::And
            | Op::Or
            | Op::Coalesce,
            a,
            b,
        ) => past(a) && past(b),
        ExprKind::Cond(a, b, c) => past(a) && past(b) && past(c),
        // Text or a number made of such, `${a}-${b}`, `a + 1`, `String(n)`,
        // and a call of the runtime's that only reads, `$debugStr(s)`.
        ExprKind::Template(_, values, _) => values.iter().all(past),
        ExprKind::Stringed(a) | ExprKind::Unary(_, a) => past(a),
        ExprKind::Binary(_, a, b) => past(a) && past(b),
        ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::is_pure) => {
            args.iter().all(past)
        }
        // A function called at once, `((value) => ..)(value$1)`: its body,
        // its parameter what it's given.
        ExprKind::Call(callee, args)
            if let ExprKind::Arrow(params, body) = &callee.kind
                && let [js::Pattern::Name(param)] = params.as_slice()
                && let [
                    Stmt {
                        kind: StmtKind::Return(Some(result)),
                        ..
                    },
                ] = body.as_slice()
                && let [given] = args.as_slice() =>
        {
            let mut plain = plain.clone();
            plain.swap_remove(param.as_str());
            past(given) && read_past_calls(result, set, &plain)
        }
        _ => e.is_made_of_constants(),
    }
}

/// Whether making `e` reads only what no call changes: an element, array or
/// object of constants, of variables nothing sets again, of imports and of
/// arrows, which capture what they read, and of such elements.
fn inert_element(e: &Expr, changing: &IndexSet<String>) -> bool {
    let props = |props: &[js::Prop]| {
        props.iter().all(|p| {
            let (js::Prop::Field(_, v)
            | js::Prop::Getter(_, v)
            | js::Prop::Setter(_, v)
            | js::Prop::Method(_, v)
            | js::Prop::Spread(v)
            | js::Prop::Keyed(_, v)
            | js::Prop::KeyedMethod(_, v)) = p;
            !matches!(p, js::Prop::Spread(_) | js::Prop::Keyed(..) | js::Prop::KeyedMethod(..))
                && inert_element(v, changing)
        })
    };
    match &e.kind {
        ExprKind::Var(v) => !changing.contains(v.as_str()),
        ExprKind::Symbol(_) | ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,
        ExprKind::Array(items) => items.iter().all(|i| inert_element(i, changing)),
        ExprKind::Object(fields, ..) => props(fields),
        // A choice of them, `open ? <IconClose /> : <IconHamburger />`.
        ExprKind::Cond(a, b, c) => [a, b, c].iter().all(|e| inert_element(e, changing)),
        ExprKind::Jsx(jsx) => {
            let tag = match &jsx.tag {
                js::JsxTag::Component(tag) => inert_element(tag, changing),
                _ => true,
            };
            tag && props(&jsx.props) && jsx.children.iter().all(|c| inert_element(c, changing))
        }
        _ => e.is_made_of_constants(),
    }
}
