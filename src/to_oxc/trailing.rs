//! Where a comment after code on its line is in the JS (ADR 0444): after
//! the last JS of its line; a line comment's where JS has the next line's
//! code after it (ADR 0733); and one inside an expression after the code
//! right before it, or after the `=` it's after (ADR 0630, amended). It
//! reads the generated code's lines and its source map, and gives where each
//! comment goes in them.

use std::collections::HashMap;

use oxc_sourcemap::SourceMap;

/// A comment after code on its line (ADR 0444): its line, its column,
/// whether code follows it there, inside an expression (ADR 0630), whether
/// what's before it there is an `=`, which it's after, before the value
/// (ADR 0630, amended), and its text.
pub(super) struct Trailing {
    pub(super) line: u32,
    pub(super) column: u32,
    pub(super) inside: bool,
    pub(super) before_value: bool,
    pub(super) text: String,
    /// Of a line comment after a function's parameter on its line, `start:
    /// f64, // DEV-only`, where the parameter starts, its line and column:
    /// it's after that parameter's JS, the list's rest on the next line
    /// (ADR 0708, amended).
    pub(super) after_param: Option<(u32, u32)>,
}

/// Where each function's parameter starts, by where it ends: a line
/// comment after it on its line is after it (ADR 0708, amended).
pub(super) fn param_starts(module: &crate::js::Module) -> HashMap<u32, u32> {
    (module.items.iter())
        .filter_map(|item| match item {
            crate::js::Item::Function(f) => Some(f),
            _ => None,
        })
        .flat_map(|f| f.param_spans.iter().filter(|s| !s.is_none()).map(|s| (s.hi, s.lo)))
        .collect()
}

/// Of a comment at `at` of its line, from `line_start`, in `text`, the
/// line and column of the parameter it's after there, past its `,`.
pub(super) fn after_param(
    text: &str,
    (line_start, at): (usize, usize),
    param_starts: &HashMap<u32, u32>,
) -> Option<(u32, u32)> {
    let code = text[line_start..at].trim_end();
    let code = code.strip_suffix(',').unwrap_or(code).trim_end();
    let &lo = param_starts.get(&((line_start + code.len()) as u32))?;
    let start = text[..lo as usize].rfind('\n').map_or(0, |i| i + 1);
    Some((text[..lo as usize].matches('\n').count() as u32, lo - start as u32))
}

/// Where in its line of JS a comment inside an expression is (ADR 0630),
/// from the column it's placed by.
#[derive(Clone, Copy)]
pub(super) enum Inside {
    /// After the name, number or string that starts there, `37 /* "%" */`.
    AfterCode,
    /// After the `=` there, before the value it gives, `NoMode = /* .. */
    /// 0b0000000` (ADR 0630, amended).
    AfterEquals,
}

/// Where each comment after code on its line goes, by the generated code's
/// line: after the line's code; inside it, at a column, after the code or
/// the `=` there; or ending it at a column, the rest of it on the next line.
#[derive(Default)]
pub(super) struct Placed<'a> {
    pub(super) after: HashMap<u32, Vec<&'a str>>,
    pub(super) inside: HashMap<u32, Vec<(u32, Inside, &'a str)>>,
    pub(super) ends: HashMap<u32, Vec<(u32, &'a str)>>,
}

/// Where each of `trailing` goes in `generated_lines`, by the map of them.
pub(super) fn placed<'a>(trailing: &'a [Trailing], map: &SourceMap, generated_lines: &[&str]) -> Placed<'a> {
    // A comment after code is after the last JS of its line (ADR 0444); of
    // a line no JS is of, a `}`'s, it's nobody's.
    let mut after: HashMap<u32, Vec<&str>> = HashMap::new();
    // One inside an expression, after the JS of the code right before it,
    // or of an `=` it's after, its line's and column's (ADR 0630).
    let mut inside: HashMap<u32, Vec<(u32, Inside, &str)>> = HashMap::new();
    // A line comment after code of its line that JS has before the code of
    // a line after it, `? "…" // ellipsis` of a conditional on one line:
    // after the code right before it, the rest of the JS's line on the next
    // (ADR 0733), so it's that code's, not all of it after the line's end.
    let mut ends: HashMap<u32, Vec<(u32, &str)>> = HashMap::new();
    for comment in trailing {
        // One right after an `=`, before the value it gives: after the
        // `=` of the JS of its line, `NoMode = /* .. */ 0b0000000`,
        // `mode = /* .. */ (given | ConcurrentMode) >>> 0` (ADR 0630,
        // amended). By the JS the map places of the code nearest after
        // it on its line, the value's, else nearest before it, the
        // declaration's: the `=` before that, else after it, the
        // statement's JS being placed where its value is or where it is.
        let equals = || {
            let of_line = || (map.get_tokens()).filter(|t| t.get_src_line() == comment.line);
            let after = of_line()
                .filter(|t| t.get_src_col() > comment.column)
                .min_by_key(|t| t.get_src_col());
            let before = of_line()
                .filter(|t| t.get_src_col() < comment.column)
                .max_by_key(|t| t.get_src_col());
            after.into_iter().chain(before).find_map(|t| {
                let line = t.get_dst_line();
                let text = generated_lines.get(line as usize)?;
                let at = in_bytes(text, t.get_dst_col());
                let found = (text[..at].rfind(" = ")).or_else(|| text[at..].find(" = ").map(|i| at + i))?;
                Some((line, found as u32 + 1))
            })
        };
        // One after a parameter: after its name, `start // DEV-only`, the
        // rest of the line, `) {`, on the next, as the list's next
        // parameter is (ADR 0708, amended).
        if let Some((line, column)) = comment.after_param
            && let Some(found) = (map.get_tokens())
                .filter(|t| t.get_src_line() == line && t.get_src_col() == column)
                .find_map(|t| {
                    let text = generated_lines.get(t.get_dst_line() as usize)?;
                    let end = word_end(text, t.get_dst_col())?;
                    text[end..]
                        .trim_start()
                        .starts_with(')')
                        .then_some((t.get_dst_line(), end as u32))
                })
        {
            ends.entry(found.0).or_default().push((found.1, &comment.text));
            continue;
        }
        if comment.inside
            && comment.before_value
            && let Some((line, column)) = equals()
        {
            inside
                .entry(line)
                .or_default()
                .push((column, Inside::AfterEquals, &comment.text));
            continue;
        }
        // The nearest before it whose JS is a name, a number or a string.
        let before = {
            let mut tokens: Vec<_> = (map.get_tokens())
                .filter(|t| t.get_src_line() == comment.line && t.get_src_col() < comment.column)
                .collect();
            tokens.sort_by_key(|t| std::cmp::Reverse(t.get_src_col()));
            tokens
                .into_iter()
                .map(|t| (t.get_dst_line(), t.get_dst_col()))
                .find(|&(line, column)| {
                    (generated_lines.get(line as usize)).is_some_and(|text| word_end(text, column).is_some())
                })
        };
        // Where its JS ends, of a line comment, when a later line's JS
        // follows it on its line: before that, and the `(`, `?`, `:` or
        // `,` it's after.
        let end = before.filter(|_| !comment.inside).and_then(|(line, column)| {
            let text = generated_lines.get(line as usize)?;
            let later = (map.get_tokens())
                .filter(|t| t.get_dst_line() == line && t.get_dst_col() > column && t.get_src_line() > comment.line)
                .map(|t| t.get_dst_col())
                .min()?;
            let head = &text[..in_bytes(text, later)];
            let cut = head
                .trim_end_matches(|c: char| c.is_whitespace() || "([{?:,".contains(c))
                .len();
            (cut > in_bytes(text, column)).then_some((line, cut as u32))
        });
        match (before, end) {
            (Some((line, column)), _) if comment.inside => {
                inside
                    .entry(line)
                    .or_default()
                    .push((column, Inside::AfterCode, &comment.text))
            }
            (_, Some((line, end))) => ends.entry(line).or_default().push((end, &comment.text)),
            _ => {
                let last = (map.get_tokens())
                    .filter(|t| t.get_src_line() == comment.line)
                    .map(|t| t.get_dst_line())
                    .max();
                if let Some(last) = last {
                    after.entry(last).or_default().push(&comment.text);
                }
            }
        }
    }
    Placed { after, inside, ends }
}

/// Where a map's `column` of `line`, in UTF-16 units, is in its bytes.
fn in_bytes(line: &str, column: u32) -> usize {
    let mut units = 0;
    for (at, c) in line.char_indices() {
        if units >= column {
            return at;
        }
        units += c.len_utf16() as u32;
    }
    line.len()
}

/// Where the name, number or string at `column` of `line` ends: none of
/// another token, `(` or `{`.
fn word_end(line: &str, column: u32) -> Option<usize> {
    let start = column as usize;
    let rest = line.get(start..)?;
    let end = match rest.chars().next()? {
        quote @ ('"' | '\'' | '`') => rest[1..].find(quote).map(|i| i + 2)?,
        _ => (rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$' || c == '.'))).unwrap_or(rest.len()),
    };
    (end > 0).then_some(start + end)
}

/// `line` of the JS with each comment after the code that starts at its
/// column: a name, a number or a string, `case 79 /* "O" */:` (ADR 0630);
/// or after the `=` there, `NoMode = /* .. */ 0b0000000` (ADR 0630, amended).
/// And where each is, in `line`, and how wide, in the map's units: what the
/// map's columns from there are moved by.
pub(super) fn with_inside_comments(line: &str, comments: &[(u32, Inside, &str)]) -> (String, Vec<(u32, u32)>) {
    let mut out = line.to_string();
    let mut at: Vec<(usize, &str)> = (comments.iter())
        .filter_map(|&(column, inside, text)| match inside {
            Inside::AfterCode => Some((word_end(line, column)?, text)),
            Inside::AfterEquals => {
                (line.get(column as usize..)?.starts_with('=')).then_some((column as usize + 1, text))
            }
        })
        .collect();
    // From the last, so each column is still where it was.
    at.sort_by_key(|&(end, _)| std::cmp::Reverse(end));
    let mut moved = Vec::new();
    for (end, text) in at {
        let inserted = format!(" {text}");
        moved.push((end as u32, inserted.encode_utf16().count() as u32));
        out.insert_str(end, &inserted);
    }
    (out, moved)
}

/// Where a block comment `c` alone on its lines, between an `=` and the
/// value on a line after it, `=\n /* .. */\n value`, goes: the line and
/// the column right before the value's first code, as if written there
/// (ADR 0630, amended). Not one after `==`, `<=` or a compound `+=`, nor
/// one of more than a line; what's between it and the `=` and the value
/// may be other comments only.
pub(super) fn own_line_before_value(
    text: &str,
    comments: &[crate::program::Comment],
    c: &crate::program::Comment,
) -> Option<(u32, u32)> {
    let written = text.get(c.lo as usize..c.hi as usize)?;
    if !written.starts_with("/*") || written.contains('\n') {
        return None;
    }
    let line_start = text[..c.lo as usize].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[c.hi as usize..]
        .find('\n')
        .map_or(text.len(), |i| c.hi as usize + i);
    if !text[line_start..c.lo as usize].trim().is_empty() || !text[c.hi as usize..line_end].trim().is_empty() {
        return None;
    }
    // Past whitespace and other comments, each way.
    let mut before = c.lo as usize;
    loop {
        let code = text[..before].trim_end();
        match comments.iter().find(|o| o.hi as usize == code.len()) {
            Some(o) => before = o.lo as usize,
            None => break,
        }
    }
    let code = text[..before].trim_end();
    let equals = code
        .strip_suffix('=')
        .is_some_and(|before| !before.ends_with(|c: char| "=!<>+-*/%&|^".contains(c)));
    let mut after = c.hi as usize;
    loop {
        let skipped = after + (text[after..].len() - text[after..].trim_start().len());
        match comments.iter().find(|o| o.lo as usize == skipped) {
            Some(o) => after = o.hi as usize,
            None => {
                after = skipped;
                break;
            }
        }
    }
    if !equals || after >= text.len() {
        return None;
    }
    let value_line_start = text[..after].rfind('\n').map_or(0, |i| i + 1);
    let line = text[..after].matches('\n').count() as u32;
    // Right before the value, so the map's tokens after it are the value's.
    let column = (after - value_line_start) as u32;
    Some((line, column.saturating_sub(1)))
}

/// Where a line comment `c` alone on its line after a conditional's
/// branch's value, before its block's `}`, goes: right after the value, as
/// if written after it on its line, `? 2 // TODO .. : phase` (ADR 0658,
/// amended). `value_ends` are where each branch's value ends; what's
/// between it and the value, and it and the `}`, may be other comments
/// only, each placed so in turn.
pub(super) fn own_line_after_value(
    text: &str,
    comments: &[crate::program::Comment],
    value_ends: &std::collections::HashSet<u32>,
    c: &crate::program::Comment,
) -> Option<(u32, u32)> {
    let written = text.get(c.lo as usize..c.hi as usize)?;
    if !written.starts_with("//") {
        return None;
    }
    let line_start = text[..c.lo as usize].rfind('\n').map_or(0, |i| i + 1);
    if !text[line_start..c.lo as usize].trim().is_empty() {
        return None;
    }
    // Past whitespace and other comments, each way.
    let mut before = c.lo as usize;
    loop {
        let code = text[..before].trim_end();
        match comments.iter().find(|o| o.hi as usize == code.len()) {
            Some(o) => before = o.lo as usize,
            None => break,
        }
    }
    let end = text[..before].trim_end().len();
    let mut after = c.hi as usize;
    loop {
        let skipped = after + (text[after..].len() - text[after..].trim_start().len());
        match comments.iter().find(|o| o.lo as usize == skipped) {
            Some(o) => after = o.hi as usize,
            None => {
                after = skipped;
                break;
            }
        }
    }
    if !value_ends.contains(&(end as u32)) || !text[after..].starts_with('}') {
        return None;
    }
    let value_line_start = text[..end].rfind('\n').map_or(0, |i| i + 1);
    let line = text[..end].matches('\n').count() as u32;
    Some((line, (end - value_line_start) as u32))
}
