//! A comment after a call's last argument, before its `)`, `f(a, b, //
//! c\n)`: there, where oxc writes a comment at a call's `)` (ADR 0708,
//! amended), as Prettier keeps it.

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
            .chain([&crate::program::Comment { lo: hi, hi, doc: false }])
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
