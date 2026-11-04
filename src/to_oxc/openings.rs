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

/// Whether `code`, what's between a comment and the next start, holds a
/// whole match arm, its pattern, `=>`, its value and its `,`, before the
/// next arm's `=>`: `Ok(h) => h,\n Err(_) => `, an arm whose value has no
/// start of its own, whose comment is its, not the next arm's (rd630).
pub(super) fn whole_arm(code: &str) -> bool {
    let mut depth = 0u32;
    let mut arrows = 0;
    let mut ended = false;
    for (at, c) in code.char_indices() {
        match c {
            '{' | '(' | '[' => depth += 1,
            ')' | ']' | '}' if depth > 0 => depth -= 1,
            ',' if depth == 0 && arrows > 0 => ended = true,
            '=' if depth == 0 && code[at..].starts_with("=>") => {
                if ended {
                    return true;
                }
                arrows += 1;
            }
            _ => {}
        }
    }
    false
}

/// Whether `text`, from a start to the next, is only a match arm's test,
/// `IMMEDIATE_PRIORITY =>`, whose comments are its body's (ADR 0590); and
/// where the next start is first in an `else if`'s block, `opens_block`,
/// of an arm that opens it, `(2, 2) => {`, its test to its `{`, past which
/// only comments are, as `comments_only` says of where the `{` is: a
/// comment above the arm is that statement's, not the test's (#595).
pub(super) fn arm_test(text: &str, opens_block: bool, comments_only: impl Fn(usize) -> bool) -> bool {
    let pattern = match text.find('{') {
        Some(open) if opens_block && comments_only(open) => text[..open].trim_end(),
        _ => text,
    };
    pattern.trim_end().ends_with("=>") && !pattern.contains([';', '{', '}', '\n'])
}

/// Whether `statement`, one of `leading_statements`, is a type alias, `pub
/// type ActivityInstance = Unknown`, its attributes first: an item with no
/// JS, so what's before it is before the next item that has (ADR 0444,
/// amended).
pub(super) fn type_alias(statement: &str) -> bool {
    item_after_visibility(statement).starts_with("type ")
}

/// Whether `statement`, one of `leading_statements`, is a `use`, `use
/// std::rc::Rc`, its attributes first: one that makes an import starts
/// where it's written, so this is one that makes none, a type's, whose
/// comments are before the next import or item (rd92).
pub(super) fn use_item(statement: &str) -> bool {
    item_after_visibility(statement).starts_with("use ")
}

/// `statement` past its attributes and its `pub`, `pub(crate)`'s too.
fn item_after_visibility(statement: &str) -> &str {
    let mut rest = statement.trim_start();
    while let Some(attribute) = rest.strip_prefix("#[") {
        let Some(end) = attribute.find(']') else { return "" };
        rest = attribute[end + 1..].trim_start();
    }
    if let Some(public) = rest.strip_prefix("pub") {
        rest = public.trim_start();
        if let Some(scoped) = rest.strip_prefix('(') {
            let Some(end) = scoped.find(')') else { return "" };
            rest = scoped[end + 1..].trim_start();
        }
    }
    rest
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

/// Whether `head`'s last line ends a `match` arm's pattern, `"focusin" =>`,
/// whose block's first statement is where the arm's own starts: a case's
/// body's, a comment before the arm before it (ADR 0630, rd119).
pub(super) fn arm_opening(head: &str) -> bool {
    let line = head[head.rfind('\n').map_or(0, |i| i + 1)..].trim();
    line.strip_suffix("=>")
        .is_some_and(|pattern| !pattern.trim().is_empty())
}

/// Whether `code`, before a statement's start, is nothing, or the receiver
/// of a method call its span starts after, `DOWN.` of `DOWN.set(true)`.
pub(super) fn receiver_only(code: &str) -> bool {
    let code = code.trim();
    code.is_empty()
        || (code.strip_suffix('.')).is_some_and(|path| {
            path.chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '.')
        })
}

/// Whether `line` ends opening a closure's body, `NAMES.with(|names| {`,
/// `move || {`: its parameters' `|`, then `{`.
pub(super) fn closure_opening(line: &str) -> bool {
    line.trim_end()
        .strip_suffix('{')
        .is_some_and(|head| head.trim_end().ends_with('|'))
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

/// Of a `start` inside a `let`'s value on its line, `let settled = match
/// &mut *chunk.state.borrow_mut() {`'s scrutinee, whose JS needn't start
/// there, a `match`'s chain at its first arm: where that `let` ends, as
/// rustfmt writes it, at the next line no deeper than its first but one of
/// its closings, `}`, `)` or `]` (rd533).
pub(super) fn let_end(text: &str, start: u32) -> Option<u32> {
    let start = start as usize;
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let first = &text[line_start..start];
    if !first.trim_start().starts_with("let ") {
        return None;
    }
    let indent = first.len() - first.trim_start().len();
    let mut at = text[start..].find('\n').map_or(text.len(), |i| start + i + 1);
    while at < text.len() {
        let line = &text[at..text[at..].find('\n').map_or(text.len(), |i| at + i)];
        let code = line.trim_start();
        let depth = line.len() - code.len();
        if !code.is_empty() && (depth < indent || depth == indent && !code.starts_with(['}', ')', ']'])) {
            break;
        }
        at += line.len() + 1;
    }
    Some(at.min(text.len()) as u32)
}
