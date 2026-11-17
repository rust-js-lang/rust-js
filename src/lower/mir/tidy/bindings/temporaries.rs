//! A temporary's name, given where tidy keeps it: `_4`, MIR's, read more
//! than once, named after what made it (ADR 0569). Split from bindings.rs
//! by concept, as a file is at most 1500 lines.

use super::*;
use crate::lower::temporary_names::{self, Role};

/// `_4`, a temporary MIR holds still, read more than once: named after
/// what it holds, the field it reads, `routes` of `tree.routes` or
/// `length` of `slice?.length`, not a positional one, `_0`, which is a
/// temporary's name, else what rust-analyzer suggests of it,
/// `trailingZeros` or `point` (`suggested`, ADR 0569); a trait object's
/// whose method is called, `receiver`; else `value`, an argument held
/// `arg`, or `tmp` of a `let` its branches set. The first of those names no
/// other in the function has, its parameters' and closures' too, else
/// `value`, `arg` or `tmp`, given apart where it must be. And each value
/// lowering spilled for a role, named at once, `value$3`, in the body's
/// naming table (`spilled`): named again by the same pick, its call's name
/// first, `pullTwo`, else its role's word (#890, #837).
/// What MIR suggests of its temporaries, the body's naming table, and what
/// an item written in the body may read (`readable_temporaries`).
type Named<'a> = (
    &'a IndexMap<js::Name, Vec<String>>,
    &'a IndexMap<String, Role>,
    &'a IndexSet<String>,
);

/// What made `value` suggests a `const` of it is named, its call's name,
/// `pullTwo` of `RangePull.pullTwo(..)` (`held_name`).
fn held_names(value: &Expr) -> Vec<String> {
    (crate::lower::mir::temporaries::held_name(value).into_iter())
        .map(|held| held.name.clone())
        .collect()
}

pub(in crate::lower::mir::tidy) fn readable_temporaries(
    params: &[js::Pattern],
    body: &mut Vec<Stmt>,
    (suggested, spilled, items_read): Named<'_>,
) {
    let mut taken: IndexSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    js::names_in(body, &mut taken);
    // A spilled value's name, picked again: of those still declared, as
    // tidy may have read one in place, and once; and named nowhere in a
    // function or closure of the body, whose own names its tidy gave
    // already, `eval$`'s `result` beside its maker's `result` (#890).
    let mut nested: IndexSet<String> = IndexSet::default();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter() {
            if let StmtKind::Function(function) = &stmt.kind {
                js::names_in(&function.body, &mut nested);
                nested.extend(function.params.iter().flat_map(|p| p.names()).map(str::to_string));
            }
        }
    });
    js::each_expr(body, &mut |e| match &e.kind {
        ExprKind::Arrow(params, inner) | ExprKind::AsyncArrow(params, inner) => {
            js::names_in(inner, &mut nested);
            nested.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
        }
        ExprKind::Function(function) => {
            js::names_in(&function.body, &mut nested);
            nested.extend(function.params.iter().flat_map(|p| p.names()).map(str::to_string));
        }
        _ => {}
    });
    let mut declared: IndexMap<String, usize> = IndexMap::default();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter() {
            if let StmtKind::Const(name, _) | StmtKind::Let(name, _) = &stmt.kind {
                *declared.entry(name.word().to_string()).or_default() += 1;
            }
        }
    });
    let spilled: Vec<(&String, &Role)> = spilled
        .iter()
        .filter(|(name, _)| declared.get(name.as_str()) == Some(&1) && !nested.contains(name.as_str()))
        .collect();
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
    let mut given: IndexMap<js::Name, String> = IndexMap::default();
    js::each_expr(body, &mut |e| {
        // A trait object's, `_5.impl.add(_5.value)`: its `receiver`.
        if let ExprKind::Call(callee, _) = &e.kind
            && let ExprKind::Member(of, _) = &callee.kind
            && let ExprKind::Member(object, key) = &of.kind
            && key == js::DICTIONARY
            && let ExprKind::Var(v) = &object.kind
            && is_temporary(v)
        {
            given
                .entry(v.clone())
                .or_insert_with(|| Role::Receiver.text().to_string());
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
                    given
                        .entry(v.clone())
                        .or_insert_with(|| Role::Children.text().to_string());
                }
            }
        }
    });
    let mut renames = Vec::new();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter() {
            match &stmt.kind {
                StmtKind::Const(name, value)
                    if !is_temporary(name)
                        && let Some((_, role)) = spilled.iter().find(|(n, _)| name.is(n))
                        && role.says_nothing() =>
                {
                    renames.push((name.clone(), held_names(value), **role));
                }
                // Not what Rust leaves `_`, which says nothing's read.
                StmtKind::Const(name, value) if (is_temporary(name) && !name.is_unread()) || is_held_arg(name) => {
                    let any = match is_held_arg(name) {
                        true => Role::Arg,
                        false => Role::Value,
                    };
                    let names = match (given.get(name), &value.kind) {
                        (Some(prop), _) => vec![prop.clone()],
                        (None, _) if is_held_arg(name) => Vec::new(),
                        // Not a positional field's, `_0`, a temporary's name.
                        (None, ExprKind::Member(_, field) | ExprKind::OptionalMember(_, field))
                            if !spelled_as_temporary(field) =>
                        {
                            vec![field.clone()]
                        }
                        // A copy of one, as it's named (#1923).
                        (None, _)
                            if let Some(ExprKind::Member(_, field) | ExprKind::OptionalMember(_, field)) =
                                crate::lower::copies::copied_source(value).map(|source| &source.kind)
                                && !spelled_as_temporary(field) =>
                        {
                            vec![field.clone()]
                        }
                        (None, _) => suggested.get(name).cloned().unwrap_or_default(),
                    };
                    // Else what made its value suggests, its call's name
                    // (#890, #837); not an argument held, which MIR's rule
                    // names `arg`.
                    let names = match names.is_empty() && !is_held_arg(name) {
                        true => held_names(value),
                        false => names,
                    };
                    renames.push((name.clone(), names, any));
                }
                // Held in a `try` that assigns it, `let _arg; try { _arg = .. }`,
                // as what it's given as is: `arg`.
                StmtKind::Let(name, _) if is_held_arg(name) => renames.push((name.clone(), Vec::new(), Role::Arg)),
                StmtKind::Let(name, _) if is_temporary(name) => {
                    renames.push((
                        name.clone(),
                        suggested.get(name).cloned().unwrap_or_default(),
                        Role::BranchSet,
                    ));
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
    js::visit_stmt_names(body, &mut |name| {
        first.entry(name.clone()).or_insert(at);
        at += 1;
    });
    renames.sort_by_key(|(from, ..)| first.get(from).copied().unwrap_or(usize::MAX));
    for (from, names, role) in renames {
        // A spilled value's name changes only to what made it suggests, its
        // call's, where that's free: else it keeps the one it has, which
        // tidy may have given again in a block apart, `result` of each arm
        // (ADR 0352), not its role's word given apart anew (#890).
        if spilled.iter().any(|(name, _)| from.is(name)) && names.iter().all(|name| taken.contains(name)) {
            continue;
        }
        let to = js::Name::declared(temporary_names::pick(names, role, &mut taken));
        // By its id: a closure's own of its name, `(_2) => _2[0]`, is
        // another variable, never this one (ADR 0902; ADR 1150).
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
    }
}
