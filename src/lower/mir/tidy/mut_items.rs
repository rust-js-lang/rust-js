//! A loop over an array's items through their `&mut`s, `for s in
//! state.iter_mut()`, `$mutItems(state)`'s handles, is a loop over its
//! indices, each item written in its place, `state[i] = ..`, as a person
//! writes it (#320, #354): of each item alone, of its `enumerate()`, its
//! index the loop's, and of its `zip` with another array, as long as the
//! shorter, which borrowck keeps either from changing meanwhile.

use super::*;

/// Each such loop of `stmts`, a block's, an index loop; `mentions`, the
/// names the function has, which its index and bound are named apart from.
pub(super) fn mut_item_loops(stmts: &mut Vec<Stmt>, mentions: &mut HashMap<String, usize>) {
    let mut i = 0;
    while i < stmts.len() {
        if let Some(made) = index_loop(&stmts[i], mentions) {
            let n = made.len();
            stmts.splice(i..i + 1, made);
            i += n;
            continue;
        }
        i += 1;
    }
}

/// `for (const s of $mutItems(a))`, `for (const [i, s] of
/// $iter($mutItems(a)).map((x, i) => [i, x]))`, its `.entries()`, or `for (const [s, c] of
/// $lazyZip($iter($mutItems(a)), b))`, its handle `s` read and written
/// only as `s.value`: the index loop of it, and of a zip, its bound
/// first, `const n = Math.min(a.length, b.length);`.
fn index_loop(stmt: &Stmt, mentions: &mut HashMap<String, usize>) -> Option<Vec<Stmt>> {
    let StmtKind::ForOf {
        label,
        pattern,
        mutable: false,
        iterable,
        body,
        keys: false,
    } = &stmt.kind
    else {
        return None;
    };
    let mut fresh = |base: &str| {
        let name = std::iter::once(base.to_string())
            .chain((1..).map(|k| format!("{base}${k}")))
            .find(|n| !mentions.contains_key(n))
            .expect("a name");
        mentions.insert(name.clone(), 1);
        name
    };
    let items_of = |e: &Expr| match &e.kind {
        ExprKind::Call(callee, args) if callee.helper_name() == Some("$mutItems") => match args.as_slice() {
            [
                array @ Expr {
                    kind: ExprKind::Var(_), ..
                },
            ] => Some(array.clone()),
            _ => None,
        },
        _ => None,
    };
    let name = |p: &Option<js::Pattern>| match p {
        Some(js::Pattern::Name(name)) => Some(name.clone()),
        _ => None,
    };
    // The array, its handle's name, the index, and of a zip, the other
    // array and what its item binds.
    let (array, handle, index, other) = match (&iterable.kind, pattern) {
        (_, js::Pattern::Name(handle)) if let Some(array) = items_of(iterable) => (array, handle.clone(), None, None),
        // Its `enumerate()`, `$iter($mutItems(a)).map((x, i) => [i, x])`.
        (_, js::Pattern::Array(parts))
            if let Some(source) = crate::lower::iterators::enumeration_source(iterable)
                && let Some(items) = crate::lower::iterators::fresh_iter_source(source)
                && let Some(array) = items_of(items)
                && let [index, handle] = parts.as_slice() =>
        {
            (array, name(handle)?, Some(name(index)?), None)
        }
        (ExprKind::Call(callee, args), js::Pattern::Array(parts))
            if callee.helper_name() == Some("$lazyZip")
                && let [stepped, b] = args.as_slice()
                && let Some(items) = crate::lower::iterators::fresh_iter_source(stepped)
                && let Some(array) = items_of(items)
                && let [handle, item] = parts.as_slice() =>
        {
            (array, name(handle)?, None, Some((other_array(b)?, item.clone()?)))
        }
        _ => return None,
    };
    let ExprKind::Var(array_name) = &array.kind else {
        return None;
    };
    // The handle only as `s.value`, never given or kept: each its item's place.
    let held = |e: &Expr| {
        e.held_by()
            .is_some_and(|of| matches!(&of.kind, ExprKind::Var(v) if *v == handle))
    };
    let mut values = 0;
    js::each_expr(body, &mut |e| values += usize::from(held(e)));
    let other_name = other.as_ref().and_then(|(b, _)| match &b.kind {
        ExprKind::Var(b) => Some(b.clone()),
        _ => None,
    });
    if values == 0
        || js::mentions_in(body, &handle) != values
        || bindings::assigns(body, array_name)
        || other_name.as_ref().is_some_and(|b| bindings::assigns(body, b))
    {
        return None;
    }
    let index = index.unwrap_or_else(|| fresh("i"));
    let mut body = body.clone();
    js::each_expr_mut(&mut body, &mut |e| {
        if held(e) {
            e.kind = ExprKind::Index(Box::new(array.clone()), Box::new(Expr::var(&index)));
        }
    });
    let length = |a: &Expr| Expr::member(a.clone(), "length");
    let mut made = Vec::new();
    let test = match &other {
        None => length(&array),
        // As long as the shorter, asked once, as Rust's zip asks it.
        Some((b, item)) => {
            let bound = fresh("n");
            let min = Expr::member(Expr::var("Math"), "min");
            let shorter = Expr::call(min, vec![length(&array), length(b)]);
            made.push(StmtKind::Const(bound.clone(), shorter).at(js::Span::NONE));
            let bound_item = match item {
                js::Pattern::Name(name) => StmtKind::Const(name.clone(), Expr::index(b.clone(), Expr::var(&index))),
                pattern => StmtKind::Destructure {
                    pattern: pattern.clone(),
                    value: Expr::index(b.clone(), Expr::var(&index)),
                    mutable: false,
                },
            };
            body.insert(0, bound_item.at(js::Span::NONE));
            Expr::var(&bound)
        }
    };
    made.push(
        StmtKind::For {
            label: label.clone(),
            name: index.clone(),
            start: Expr::int(0),
            test: Expr::bin(Op::Lt, Expr::var(&index), test),
            step: js::Step::Up,
            body,
        }
        .at(stmt.span),
    );
    Some(made)
}

/// The array a zip steps beside the items, `b.slice()`, a copy of it the
/// zip owns, which nothing else reads, or `$iter(b)`: `b`. Not a variable
/// alone, which may hold an iterator.
fn other_array(b: &Expr) -> Option<Expr> {
    let of: &Expr = match &b.kind {
        ExprKind::Call(callee, args) if args.is_empty() => match &callee.kind {
            ExprKind::Member(of, slice) if slice == "slice" => of,
            _ => return None,
        },
        _ => crate::lower::iterators::fresh_iter_source(b)?,
    };
    matches!(of.kind, ExprKind::Var(_)).then(|| of.clone())
}
