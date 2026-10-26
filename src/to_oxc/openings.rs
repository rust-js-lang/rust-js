//! What the Rust before a statement's JS opens or ends that has no JS of
//! its own, so a comment before it is the statement's: whole statements,
//! and a block's opening, an `unsafe`'s, a macro's or a `match`'s.

/// The whole statements `code` starts with, each trimmed, each ending in a
/// `;` outside its brackets, and what's after them: the next's start, or
/// from where a bracket closes one it didn't open, its block's `}`. A
/// bracket or `;` in a string is counted as code's, which only ever leaves
/// fewer of them `let`s, or what's after them with a `;` or `}`.
pub(super) fn leading_statements(code: &str) -> (Vec<&str>, &str) {
    let (mut statements, mut depth, mut from) = (Vec::new(), 0u32, 0);
    for (at, c) in code.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => match depth.checked_sub(1) {
                Some(inside) => depth = inside,
                None => break,
            },
            ';' if depth == 0 => {
                statements.push(code[from..at].trim());
                from = at + 1;
            }
            _ => {}
        }
    }
    (statements, &code[from..])
}

/// Whether `head`, the code before a `{`, ends in `unsafe`, the keyword,
/// whose block is its code alone in JS.
pub(super) fn unsafe_opening(head: &str) -> bool {
    head.strip_suffix("unsafe")
        .is_some_and(|before| !before.ends_with(|c: char| c.is_alphanumeric() || c == '_'))
}

/// Whether `head`, the code before a `{`, is a `match` of its scrutinee,
/// `match &new_last`, the keyword first.
pub(super) fn match_opening(head: &str) -> bool {
    let line = head[head.rfind('\n').map_or(0, |i| i + 1)..].trim_start();
    line.strip_prefix("match")
        .is_some_and(|scrutinee| scrutinee.starts_with(char::is_whitespace))
}

/// Whether `line`, to its end, opens a macro's body on a line of its own,
/// `thread_local! {`, `jsx! {`: a path, `!` and `{`, whose code is the
/// program's text (ADR 0621).
pub(super) fn macro_opening(line: &str) -> bool {
    let line = line[line.rfind('\n').map_or(0, |i| i + 1)..].trim();
    line.strip_suffix('{')
        .and_then(|head| head.trim_end().strip_suffix('!'))
        .is_some_and(|path| !path.is_empty() && path.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':'))
}
