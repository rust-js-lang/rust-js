//! A comment after a call's last argument, before its `)`, `f(a, b, //
//! c\n)`: there, where oxc writes a comment at a call's `)` (ADR 0708,
//! amended), as Prettier keeps it; and where each argument and object
//! literal's field starts, which a comment before it is before (ADR 0708).

use super::*;

/// Where each call and `new` of arguments that says starts and ends.
pub(super) fn argument_ends(module: &Module) -> Vec<(u32, u32)> {
    let mut ends = Vec::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_expr_mut(&mut stmts, &mut |e| {
            if let ExprKind::Call(_, args) | ExprKind::New(_, args) = &e.kind
                && !e.span.is_none()
                && !args.is_empty()
            {
                ends.push((e.span.lo, e.span.hi));
            }
        });
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => of(vec![StmtKind::Expr(c.value.clone()).at(js::Span::NONE)]),
            js::Item::Statements(stmts) => of(stmts.clone()),
            js::Item::Function(f) => of(f.body.clone()),
            js::Item::Namespace(n) => n.methods.iter().for_each(|method| of(method.body.clone())),
        }
    }
    ends
}

/// Where `c` goes of the innermost call it's after the last argument of,
/// only what closes the call after it, `,` and `)`, and other comments:
/// at its `)`, the call's end's last byte.
pub(super) fn end_of(
    c: &crate::program::Comment,
    ends: &[(u32, u32)],
    sources: &crate::program::Sources,
    comments: &[crate::program::Comment],
) -> Option<u32> {
    let closing = |hi: u32| {
        let mut code = c.hi;
        (comments.iter())
            .filter(|other| other.lo >= c.hi && other.hi <= hi)
            .chain([&crate::program::Comment {
                lo: hi,
                hi,
                doc: false,
                inner: None,
                written: true,
            }])
            .all(|other| {
                let between = sources.text.get(code as usize..other.lo as usize);
                code = other.hi;
                between.is_some_and(|gap| gap.chars().all(|ch| ch.is_whitespace() || ch == ',' || ch == ')'))
            })
    };
    (ends.iter())
        .filter(|&&(lo, hi)| lo < c.lo && c.hi < hi && closing(hi))
        .min_by_key(|&&(lo, hi)| hi - lo)
        .map(|&(_, hi)| hi - 1)
}

/// `c`, which `end_of` places, as oxc writes it there: on a line of its
/// own, before the `)`.
pub(super) fn before_parenthesis(
    c: &crate::program::Comment,
    ends: &[(u32, u32)],
    sources: &crate::program::Sources,
    comments: &[crate::program::Comment],
) -> oxc_ast::Comment {
    use oxc_ast::{CommentKind, CommentPosition};
    let kind = match sources.text[c.lo as usize..c.hi as usize].starts_with("/*") {
        true => CommentKind::MultiLineBlock,
        false => CommentKind::Line,
    };
    let mut made = oxc_ast::Comment::new(c.lo, c.hi, kind);
    made.attached_to = end_of(c, ends, sources, comments).expect("placed");
    made.position = CommentPosition::Leading;
    made.set_preceded_by_newline(true);
    made.set_followed_by_newline(true);
    made
}

/// Where each call's and `new`'s argument starts, and each object literal's
/// field's value, of those that say, and where what they're parts of
/// starts: what a comment before one in the Rust, inside what it's part
/// of, is before in the JS, as oxc writes it (ADR 0708); and whether it's a field.
pub(super) fn part_starts(module: &Module) -> std::collections::HashMap<u32, (u32, bool)> {
    let mut starts = std::collections::HashMap::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_expr_mut(&mut stmts, &mut |e| {
            let (spans, field): (Vec<js::Span>, bool) = match &e.kind {
                ExprKind::Call(_, args) | ExprKind::New(_, args) => (args.iter().map(|arg| arg.span).collect(), false),
                ExprKind::Object(props, layout) => {
                    let values = props.iter().filter_map(|prop| match prop {
                        // A method's too, its arrow's (ADR 0882, amended);
                        // where its literal writes its field (rd296).
                        js::Prop::Field(key, value) | js::Prop::Method(key, value) => {
                            Some(layout.start(key).map_or(value.span, |at| js::Span {
                                lo: at,
                                hi: value.span.hi.max(at + 1),
                            }))
                        }
                        _ => None,
                    });
                    (values.collect(), true)
                }
                _ => (Vec::new(), false),
            };
            // Of two parts at one start, the outer's, whose whole a comment before them may be inside, a
            // struct literal's field's, `parent: x.unwrap()`, not its call's receiver's (ADR 0708, amended).
            if !e.span.is_none() {
                for s in spans.into_iter().filter(|s| !s.is_none()) {
                    starts.entry(s.lo).or_insert((e.span.lo, field));
                }
            }
        });
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => of(vec![StmtKind::Expr(c.value.clone()).at(js::Span::NONE)]),
            js::Item::Statements(stmts) => of(stmts.clone()),
            js::Item::Function(f) => of(f.body.clone()),
            js::Item::Namespace(n) => n.methods.iter().for_each(|method| of(method.body.clone())),
        }
    }
    // A re-export's names, each where its `use` writes it, whose comments
    // on lines of their own are before it, as a field's are (rd338).
    for reexport in &module.reexports {
        for (name, _) in reexport
            .written
            .iter()
            .filter(|(name, _)| !name.is_none() && name.lo > reexport.span.lo)
        {
            starts.entry(name.lo).or_insert((reexport.span.lo, true));
        }
    }
    starts
}
