//! A temporary's name, given where tidy keeps it: `_4`, MIR's, read more
//! than once, named after what made it (ADR 0569). Split from bindings.rs
//! by concept, as a file is at most 1500 lines.

use super::*;
use crate::lower::temporary_names::Role;

/// `_4`, a temporary MIR holds still, read more than once: named after
/// what it holds, the field it reads, `routes` of `tree.routes` or
/// `length` of `slice?.length`, not a positional one, `_0`, which is a
/// temporary's name, else what rust-analyzer suggests of it,
/// `trailingZeros` or `point` (`suggested`, ADR 0569); a trait object's
/// whose method is called, `receiver`; else `value`, an argument held
/// `arg`, or `tmp` of a `let` its branches set. Each is made a name not
/// spelled yet that carries those words (`Name::suggested`), which
/// preparation's one speller spells the first its scope lets it be, else
/// `value`, `arg` or `tmp`, given apart where it must be: no word table
/// of tidy's own (#1681 change 3). A value lowering spilled for a role is
/// suggested where lowering spills it (`spill`, #890).
pub(in crate::lower::mir::tidy) fn readable_temporaries(
    body: &mut Vec<Stmt>,
    suggested: &IndexMap<js::Name, Vec<String>>,
) {
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
                        true => crate::lower::mir::temporaries::held_names(value),
                        false => names,
                    };
                    renames.push((name.clone(), name.suggested(names, any.text())));
                }
                // Held in a `try` that assigns it, `let _arg; try { _arg = .. }`,
                // as what it's given as is: `arg`.
                StmtKind::Let(name, _) if is_held_arg(name) => {
                    renames.push((name.clone(), name.suggested(Vec::new(), Role::Arg.text())))
                }
                StmtKind::Let(name, _) if is_temporary(name) => {
                    let names = suggested.get(name).cloned().unwrap_or_default();
                    renames.push((name.clone(), name.suggested(names, Role::BranchSet.text())));
                }
                _ => {}
            }
        }
    });
    for (from, to) in renames {
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
