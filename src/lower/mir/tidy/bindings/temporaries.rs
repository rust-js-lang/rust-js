//! A temporary's name, given where tidy keeps it: `_4`, MIR's, read more
//! than once, named after what made it (ADR 0569). Split from bindings.rs
//! by concept, as a file is at most 1500 lines.

use super::*;

/// `_4`, a temporary MIR holds still, read more than once: named after
/// what it holds, the field it reads, `routes` of `tree.routes` or
/// `length` of `slice?.length`, not a positional one, `_0`, which is a
/// temporary's name, else what rust-analyzer suggests of it,
/// `trailingZeros` or `point` (`suggested`, ADR 0569); a trait object's
/// whose method is called, `receiver`; else `value`, an argument held
/// `arg`, or `tmp` of a `let` its branches set. The first of those names no
/// other in the function has, its parameters' and closures' too, else
/// `value`, `arg` or `tmp`, given apart where it must be.
pub(in crate::lower::mir::tidy) fn readable_temporaries(
    params: &[js::Pattern],
    body: &mut Vec<Stmt>,
    (suggested, items_read): (&IndexMap<String, Vec<String>>, &IndexSet<String>),
) {
    let mut taken: IndexSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    js::names_in(body, &mut taken);
    // Nor what an item written in the body, a function or a `const`, may
    // read where nothing here shows it, as it's placed after: a name of
    // the module's, which no local takes (ADR 0267), so `const note =
    // id(-1)` hid the `note` a function `h0` written beside it calls
    // (nightly 43133510260's generated program 12461955). Of a body with
    // none, what it reads is all that's read in it, and a name of the
    // module's it doesn't read hides nothing.
    taken.extend(items_read.iter().cloned());
    // And the name each function another module gives this one is imported
    // by, which the link step keeps where no local takes it: a suggested
    // `prepare` of `prepare(3)` would push the import apart, `prepare$1`.
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Symbol(symbol) = &e.kind {
            taken.insert(symbol.export.clone());
        }
    });
    // What an element is given each as, `href={_5}` or a child, `{_6}`: the
    // prop's name, `href`, or `children`.
    let mut given: IndexMap<String, String> = IndexMap::default();
    js::each_expr(body, &mut |e| {
        // A trait object's, `_5.impl.add(_5.value)`: its `receiver`.
        if let ExprKind::Call(callee, _) = &e.kind
            && let ExprKind::Member(of, _) = &callee.kind
            && let ExprKind::Member(object, key) = &of.kind
            && key == js::DICTIONARY
            && let ExprKind::Var(v) = &object.kind
            && is_temporary(v)
        {
            given.entry(v.clone()).or_insert_with(|| "receiver".to_string());
        }
        if let ExprKind::Jsx(jsx) = &e.kind {
            for prop in &jsx.props {
                if let js::Prop::Field(key, value) = prop
                    && let ExprKind::Var(v) = &value.kind
                    && (is_temporary(v) || is_held_arg(v))
                    && key.chars().all(|c| c.is_ascii_alphanumeric())
                {
                    given.entry(v.clone()).or_insert_with(|| key.clone());
                }
            }
            for child in &jsx.children {
                if let ExprKind::Var(v) = &child.kind
                    && (is_temporary(v) || is_held_arg(v))
                {
                    given.entry(v.clone()).or_insert_with(|| "children".to_string());
                }
            }
        }
    });
    let mut renames = Vec::new();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter() {
            match &stmt.kind {
                StmtKind::Const(name, value) if is_temporary(name) || is_held_arg(name) => {
                    let any = match is_held_arg(name) {
                        true => "arg",
                        false => "value",
                    };
                    let names = match (given.get(name), &value.kind) {
                        (Some(prop), _) => vec![prop.clone()],
                        (None, _) if is_held_arg(name) => Vec::new(),
                        // Not a positional field's, `_0`, a temporary's name.
                        (None, ExprKind::Member(_, field) | ExprKind::OptionalMember(_, field))
                            if !is_temporary(field) =>
                        {
                            vec![field.clone()]
                        }
                        (None, _) => suggested.get(name).cloned().unwrap_or_default(),
                    };
                    renames.push((name.clone(), names, any));
                }
                // Held in a `try` that assigns it, `let _arg; try { _arg = .. }`,
                // as what it's given as is: `arg`.
                StmtKind::Let(name, _) if is_held_arg(name) => renames.push((name.clone(), Vec::new(), "arg")),
                StmtKind::Let(name, _) if is_temporary(name) => {
                    renames.push((name.clone(), suggested.get(name).cloned().unwrap_or_default(), "tmp"));
                }
                _ => {}
            }
        }
    });
    // In the order they're first read, as the program reads: one made in a
    // `try` is named before a `let` the function declares first and sets
    // later, `let next; .. next = $next(z);` (ADR 0705).
    let mut first = IndexMap::default();
    let mut at = 0;
    js::visit_stmts(body, &mut |name| {
        first.entry(name.to_string()).or_insert(at);
        at += 1;
    });
    renames.sort_by_key(|(from, ..)| first.get(from).copied().unwrap_or(usize::MAX));
    for (from, names, any) in renames {
        // The first name of what it holds no other has, else what any is
        // named, `value`, `arg` or `tmp`, given apart where it must be.
        let base = (names.into_iter())
            .find(|name| !taken.contains(name))
            .unwrap_or_else(|| any.to_string());
        let to = crate::names::fresh_in(&mut taken, &base);
        // Not a closure's own of its name, `(_2) => _2[0]` (ADR 0902).
        js::outside_shadows(body, &from, |body| {
            js::statement_lists(body, &mut |list| {
                for stmt in list.iter_mut() {
                    if let StmtKind::Const(name, _) | StmtKind::Let(name, _) = &mut stmt.kind
                        && *name == from
                    {
                        *name = to.clone();
                    }
                }
            });
            js::each_expr_mut(body, &mut |e| {
                if let ExprKind::Var(v) = &mut e.kind
                    && *v == from
                {
                    *v = to.clone();
                }
            });
        });
    }
}
