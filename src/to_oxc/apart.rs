//! Where a blank line is between a module's statements: where the Rust has
//! one between the code of one and the next's, or its comments (`tops`,
//! `comments_of`), and none where it has none (ADR 0444, amended).

use oxc_ast::ast::Statement;
use oxc_ast_visit::Visit;

/// `comments`, each of `body`'s statements but the first with a blank line
/// before it where the Rust has one between the code of the one before
/// and its own, and its comments, `tops` (`comments_of`), and none where
/// it has none (ADR 0444, amended); and which the Rust told so, of two in
/// one file, the code of the first ending before the second's starts,
/// which `format::spaced` leaves as they are. Of the rest, a statement
/// rust-js makes of no code of its own, or of another's, `spaced` sets
/// them apart as a person does (ADR 0303). `end` is where the text of the
/// comments ends, where a blank line's empty one is.
pub(super) fn apart(
    body: &[Statement<'_>],
    mut comments: Vec<oxc_ast::Comment>,
    tops: &std::collections::HashMap<u32, u32>,
    sources: &crate::program::Sources,
    end: u32,
) -> (Vec<oxc_ast::Comment>, Vec<bool>) {
    use oxc_span::GetSpan;
    // Where each statement's code ends in the Rust: the last its nodes are.
    struct Reach {
        end: u32,
        limit: u32,
    }
    impl<'a> Visit<'a> for Reach {
        fn enter_node(&mut self, kind: oxc_ast::AstKind<'a>) {
            let span = kind.span();
            if span.start < span.end && span.end <= self.limit {
                self.end = self.end.max(span.end);
            }
        }
    }
    let limit = sources.text.len() as u32;
    let ends: Vec<u32> = (body.iter())
        .map(|statement| {
            let mut reach = Reach { end: 0, limit };
            reach.visit_statement(statement);
            reach.end
        })
        .collect();
    // The file each place is in, by where each starts.
    let mut files = Vec::new();
    let mut start = 0;
    for file in &sources.files {
        files.push((start, start + file.text.len() as u32));
        start += file.text.len() as u32 + 1;
    }
    let file_of = |at: u32| files.iter().position(|&(lo, hi)| lo <= at && at <= hi);
    let mut told = vec![false; body.len()];
    for i in 1..body.len() {
        let (code_end, start) = (ends[i - 1], body[i].span().start);
        let top = tops.get(&start).map_or(start, |&first| first.min(start));
        // An import printed after one whose code is written below it, a
        // `js::import!`'s where ADR 1063 orders it, after what's named: the
        // blank line above it in the Rust isn't one between them, so none
        // (rd666, ADR 1069). Not the second of one `use`'s, at its start.
        if start != 0 && body[i - 1].span().start > top && matches!(body[i], Statement::ImportDeclaration(_)) {
            comments.retain(|c| !(c.attached_to == start && c.span.is_empty()));
        }
        if code_end == 0
            || start == 0
            || code_end > top
            || file_of(code_end).is_none()
            || file_of(code_end) != file_of(top)
        {
            continue;
        }
        // Of one line, `#[derive(Debug, Serialize)]`'s functions, the Rust
        // tells nothing of lines between them.
        let lines: Vec<&str> = sources.text[code_end as usize..top as usize].split('\n').collect();
        if lines.len() < 2 {
            continue;
        }
        told[i] = true;
        let blank = lines.len() > 2 && lines[1..lines.len() - 1].iter().any(|line| line.trim().is_empty());
        // Its pieces' first, a blank line if it's one, before its comments:
        // one `comments_of` saw right before it, or else one written here,
        // where what's between has no JS, a type, an `impl`'s opening.
        let first = comments.iter().position(|c| c.attached_to == start);
        let had = first.is_some_and(|at| comments[at].span.is_empty());
        if blank && !had {
            let mut made = oxc_ast::Comment::new(end, end, oxc_ast::CommentKind::Line);
            made.attached_to = start;
            made.position = oxc_ast::CommentPosition::Leading;
            made.set_preceded_by_newline(true);
            made.set_followed_by_newline(true);
            comments.insert(first.unwrap_or(comments.len()), made);
        }
    }
    (comments, told)
}
