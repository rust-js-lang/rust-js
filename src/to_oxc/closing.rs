//! A comment on lines of its own after a struct literal's last field,
//! before its `}`, `Info { name, start, // c\n }`: there in the JS's
//! object too, on lines of its own (ADR 0708, amended), which oxc's printer
//! writes no comment before, so its line is split there, as a conditional's
//! branch's is (ADR 0658).

use super::*;

/// Each object literal's `}`, by where it is, and the comments before it
/// on lines of their own, as written.
pub(super) type Closing = Vec<(u32, Vec<String>)>;

/// Where each object literal of fields starts and ends.
pub(super) fn object_ends(module: &Module) -> Vec<(u32, u32)> {
    let mut ends = Vec::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_expr_mut(&mut stmts, &mut |e| {
            if let ExprKind::Object(props, _) = &e.kind
                && !e.span.is_none()
                && !props.is_empty()
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

/// Where the `}` is of the innermost object literal `c` is after the last
/// field of, only what closes it after it, `,` and `}`, and other comments.
pub(super) fn end_of(
    c: &crate::program::Comment,
    ends: &[(u32, u32)],
    sources: &crate::program::Sources,
    comments: &[crate::program::Comment],
) -> Option<u32> {
    let closing = |hi: u32| {
        let mut code = c.hi;
        sources.text.get(hi as usize - 1..hi as usize) == Some("}")
            && (comments.iter())
                .filter(|other| other.lo >= c.hi && other.hi < hi)
                .chain([&crate::program::Comment {
                    lo: hi - 1,
                    hi,
                    doc: false,
                }])
                .all(|other| {
                    let between = sources.text.get(code as usize..other.lo as usize);
                    code = other.hi;
                    between.is_some_and(|gap| gap.chars().all(|ch| ch.is_whitespace() || ch == ','))
                })
    };
    (ends.iter())
        .filter(|&&(lo, hi)| lo < c.lo && c.hi < hi && closing(hi))
        .min_by_key(|&&(lo, hi)| hi - lo)
        .map(|&(_, hi)| hi - 1)
}

/// Of `own_line`, each comment on lines of its own, those before an
/// object's `}` (`end_of`), each put in `closing` by that `}`: the rest,
/// and where each taken starts, which nothing else places.
pub(super) fn taken<'c>(
    own_line: Vec<&'c crate::program::Comment>,
    (ends, closing): (&[(u32, u32)], &mut Closing),
    sources: &crate::program::Sources,
    comments: &[crate::program::Comment],
) -> (Vec<&'c crate::program::Comment>, Vec<u32>) {
    let mut rest = Vec::new();
    let mut taken = Vec::new();
    for c in own_line {
        let Some(end) = end_of(c, ends, sources, comments) else {
            rest.push(c);
            continue;
        };
        taken.push(c.lo);
        let written = sources.text[c.lo as usize..c.hi as usize].to_string();
        match closing.iter_mut().find(|(at, _)| *at == end) {
            Some((_, texts)) => texts.push(written),
            None => closing.push((end, vec![written])),
        }
    }
    (rest, taken)
}

/// Each of `closing`, an object's `}` and the comments before it, split in
/// `generated_lines` where it is: its line, by the map, and the column of
/// the `}` its last source byte is printed as, in `splits`.
pub(super) fn split<'c>(
    closing: &'c Closing,
    map: &oxc_sourcemap::SourceMap,
    generated_lines: &[&str],
    offset: &dyn Fn(u32, u32) -> Option<u32>,
    splits: &mut std::collections::HashMap<u32, Vec<(u32, &'c [String])>>,
) {
    for (end, texts) in closing {
        let found = (map.get_tokens())
            .filter(|t| offset(t.get_src_line(), t.get_src_col()) == Some(*end))
            .map(|t| (t.get_dst_line(), t.get_dst_col()))
            .find(|&(line, column)| {
                (generated_lines.get(line as usize))
                    .and_then(|text| text.get(column as usize..))
                    .is_some_and(|rest| rest.starts_with('}'))
            });
        if let Some((line, column)) = found {
            splits.entry(line).or_default().push((column, texts.as_slice()));
        }
    }
}

/// `texts`, each a line of its own in `code`: how many lines.
pub(super) fn lines_of(texts: &[String], code: &mut String) -> u32 {
    for text in texts {
        code.push_str(text);
        code.push('\n');
    }
    texts.len() as u32
}
