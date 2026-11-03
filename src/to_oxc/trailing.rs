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
    /// Of a block comment ending a line whose next line goes on with a
    /// call chain's link, `.d() /* .. */` then `.r(form)`: that line, its
    /// comment before the link's `.` (rd63); and the column past the link's
    /// `(`, where a setter's value is, `internals /* .. */` then
    /// `.set_p(priority)`, its comment before the `=`.
    pub(super) before_link: Option<(u32, u32)>,
    /// Of a block comment on a `_` arm's pattern, `_ /* """ "{" .. */ => {`:
    /// after its `default:`, which no code of its line maps to, before the
    /// clause's `{` (rd525), `default: /* .. */ {`, as upstream writes it.
    pub(super) of_default: bool,
    /// How many brackets the code before it on its line ends closing, `]`
    /// of `[9.0, 13.0, 27.0, 32.0]; // Tab, ..`: the JS's lines of only
    /// those after its last JS, of an array the JS writes an item a line,
    /// are its line's too, so it's after the statement, not its last item
    /// (rd605).
    pub(super) closes: u32,
    /// Of a line comment after only an argument's closing brackets and its
    /// `,`, `}, // defaults to true` of an `if` given as one, which no JS
    /// of its line maps to: the line its last bracket opened on, whose
    /// lines' JS, the argument's, it's after (rd615).
    pub(super) opened_on: Option<u32>,
    /// Of a block comment after a function's parameters and its return
    /// type, before its body's `{`, `) -> Promise<ReplyBody> /* .. */ {`:
    /// where its last parameter starts, its line and column, the JS of
    /// whose list's `)` it's after, `options) /* .. */ {`, as upstream has
    /// it (rd641).
    pub(super) after_params: Option<(u32, u32)>,
}

/// Of a comment at `at` in `text` after only closing brackets and a `,` on
/// its line, from `line_start`: the line its last bracket, the argument's,
/// was opened on, counting back past the brackets between.
/// Strings and comments are read as code: a bracket in one makes another
/// line, or none, its JS's, and the comment is placed by it or lost, as
/// before (rd615).
pub(super) fn opened_on(text: &str, (line_start, at): (usize, usize)) -> Option<u32> {
    let code = text[line_start..at].trim();
    let closers = code.strip_suffix(',')?;
    if closers.is_empty() || !closers.chars().all(|c| ")]}".contains(c)) {
        return None;
    }
    // From the last, the argument's own, `)` of `}),`.
    let comma = line_start + text[line_start..at].rfind(',')?;
    let mut depth = 0usize;
    for (i, c) in text[..comma].char_indices().rev() {
        match c {
            ')' | ']' | '}' => depth += 1,
            '(' | '[' | '{' => {
                depth = depth.checked_sub(1)?;
                // An `else`'s block, `} else {`, is of the `if` before it,
                // whose block's `}` is next.
                let line = text[..i].rfind('\n').map_or(0, |at| at + 1);
                if depth == 0 && !text[line..i].trim_start().starts_with('}') {
                    return Some(text[..i].matches('\n').count() as u32);
                }
            }
            _ => {}
        }
    }
    None
}

/// Where each function's parameter starts, by where it ends: a line
/// comment after it on its line is after it (ADR 0708, amended). A type's
/// methods' too, a class's constructor function's, `function
/// RequestInstance(.., keepDebugAlive, // DEV-only` (rd554). And where
/// each function's name starts, by where it ends: a block comment after
/// the return type of one of no parameter is after its `()` (#506). And
/// where each name a re-export's `use` lists is written, by where it ends,
/// `use_state, // the hook`, the last's: after its name, before the list's
/// `}` (rd338).
pub(super) struct ParamStarts {
    by_end: HashMap<u32, u32>,
    names: HashMap<u32, u32>,
}

pub(super) fn param_starts(module: &crate::js::Module) -> ParamStarts {
    let functions = || {
        (module.items.iter()).flat_map(|item| match item {
            crate::js::Item::Function(f) => std::slice::from_ref(f),
            crate::js::Item::Namespace(namespace) => &namespace.methods[..],
            crate::js::Item::Const(_) | crate::js::Item::Statements(_) => &[],
        })
    };
    let listed = (module.reexports.iter())
        .flat_map(|reexport| reexport.written.iter().map(|&(_, alias)| alias))
        .filter(|alias| !alias.is_none());
    ParamStarts {
        by_end: functions()
            .flat_map(|f| f.param_spans.iter().filter(|s| !s.is_none()).map(|s| (s.hi, s.lo)))
            .chain(listed.map(|alias| (alias.hi, alias.lo)))
            .collect(),
        names: functions()
            .filter(|f| !f.name_span.is_none())
            .map(|f| (f.name_span.hi, f.name_span.lo))
            .collect(),
    }
}

/// Of a comment at `at` of its line, from `line_start`, in `text`, the
/// line and column of the parameter it's after there, past its `,`.
pub(super) fn after_param(
    text: &str,
    (line_start, at): (usize, usize),
    param_starts: &ParamStarts,
) -> Option<(u32, u32)> {
    let code = text[line_start..at].trim_end();
    let code = code.strip_suffix(',').unwrap_or(code).trim_end();
    let &lo = param_starts.by_end.get(&((line_start + code.len()) as u32))?;
    let start = text[..lo as usize].rfind('\n').map_or(0, |i| i + 1);
    Some((text[..lo as usize].matches('\n').count() as u32, lo - start as u32))
}

/// Of a block comment from `at` to `end` of its line, which ends at
/// `line_end`, in `text`, after a function's last parameter, the list's
/// `)` and its return type, with the function's body's `{` after it: the
/// line and column that parameter starts at (rd641). Of a function of no
/// parameter, `fn now() -> f64 /* .. */ {`, its name's (#506).
pub(super) fn after_params(
    text: &str,
    (at, end, line_end): (usize, usize, usize),
    param_starts: &ParamStarts,
) -> Option<(u32, u32)> {
    if !text[end..line_end].trim_start().starts_with('{') {
        return None;
    }
    let returned = |past: &str| {
        past.strip_prefix(')').is_some_and(|returned| {
            let returned = returned.trim();
            returned.is_empty() || returned.starts_with("->") && !returned.contains(['{', '}', ';'])
        })
    };
    let after_param = |hi: usize| returned(text[hi..at].trim_start_matches(|c: char| c.is_whitespace() || c == ','));
    // After a name, its generics, if any, and an empty `(` `)`.
    let after_name = |hi: usize| {
        let past = text[hi..at].trim_start();
        let past = match past.strip_prefix('<') {
            Some(generics) => match generics_end(generics) {
                Some(end) => &generics[end..],
                None => return false,
            },
            None => past,
        };
        (past.trim_start().strip_prefix('(')).is_some_and(|inside| returned(inside.trim_start()))
    };
    let (_, &lo) = (param_starts.by_end.iter())
        .filter(|&(&hi, _)| (hi as usize) < at && after_param(hi as usize))
        .chain((param_starts.names.iter()).filter(|&(&hi, _)| (hi as usize) < at && after_name(hi as usize)))
        .max_by_key(|&(&hi, _)| hi)?;
    let start = text[..lo as usize].rfind('\n').map_or(0, |i| i + 1);
    Some((text[..lo as usize].matches('\n').count() as u32, lo - start as u32))
}

/// Of generics past their `<`, where their `>` ends them; an arrow's,
/// `F: Fn() -> u8`, ends nothing.
fn generics_end(generics: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut previous = ' ';
    for (i, c) in generics.char_indices() {
        match c {
            '<' => depth += 1,
            '>' if previous != '-' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        previous = c;
    }
    None
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
    /// Before the code that starts there, of a comment that has none before
    /// it on its line, `.r(/* requestFormReset */ form)` (rd63).
    BeforeCode,
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
        // parameter is (ADR 0708, amended); and after a re-export's last
        // name, before its list's `}` (rd338).
        if let Some((line, column)) = comment.after_param
            && let Some(found) = (map.get_tokens())
                .filter(|t| t.get_src_line() == line && t.get_src_col() == column)
                .find_map(|t| {
                    let text = generated_lines.get(t.get_dst_line() as usize)?;
                    let end = word_end(text, t.get_dst_col())?;
                    text[end..]
                        .trim_start()
                        .starts_with([')', '}'])
                        .then_some((t.get_dst_line(), end as u32))
                })
        {
            ends.entry(found.0).or_default().push((found.1, &comment.text));
            continue;
        }
        // One after a function's parameters, before its body: before the
        // `{` after the JS of its last parameter's list's `)` (rd641).
        if let Some((line, column)) = comment.after_params
            && let Some(found) = (map.get_tokens())
                .filter(|t| t.get_src_line() == line && t.get_src_col() == column)
                .find_map(|t| {
                    let first = t.get_dst_line() as usize;
                    (first..generated_lines.len().min(first + 8)).find_map(|at| {
                        let text = generated_lines[at];
                        let from = if at == first {
                            in_bytes(text, t.get_dst_col())
                        } else {
                            0
                        };
                        let open = text.get(from..)?.find(") {")? + from + 2;
                        Some((at as u32, open as u32))
                    })
                })
        {
            inside
                .entry(found.0)
                .or_default()
                .push((found.1, Inside::BeforeCode, &comment.text));
            continue;
        }
        // One ending the line before a setter's link, of the place it
        // sets: after that place, before its `=`, found back from the JS of
        // the value it's given, `internals.p /* CurrentUpdatePriority */ =
        // priority`, as upstream has it.
        // Its value's first JS, at the link's `(` or past it, inside a call
        // the JS has none of, `i32::from(DISCRETE_EVENT_PRIORITY)`'s.
        if let Some((next, value)) = comment.before_link
            && let Some(first) = (map.get_tokens())
                .filter(|t| t.get_src_line() == next && t.get_src_col() >= value)
                .map(|t| t.get_src_col())
                .min()
            && let Some(found) = (map.get_tokens())
                .filter(|t| t.get_src_line() == next && t.get_src_col() == first)
                .find_map(|t| {
                    let text = generated_lines.get(t.get_dst_line() as usize)?;
                    let place = text[..in_bytes(text, t.get_dst_col())].trim_end().strip_suffix('=')?;
                    let assigned = !place.ends_with(|c: char| "=!<>+-*/%&|^?".contains(c));
                    assigned.then(|| (t.get_dst_line(), place.trim_end().len() as u32 + 1))
                })
        {
            inside
                .entry(found.0)
                .or_default()
                .push((found.1, Inside::BeforeCode, &comment.text));
            continue;
        }
        // One ending a call chain's link's line, before the `.` of the
        // next's, found back from that line's first code, past its `(` and
        // its name: `internals.d /* CurrentDispatcher */.r(form)` (rd63).
        if let Some((next, _)) = comment.before_link
            && let Some(found) = (map.get_tokens())
                .filter(|t| t.get_src_line() == next)
                .min_by_key(|t| t.get_src_col())
                .and_then(|t| {
                    let text = generated_lines.get(t.get_dst_line() as usize)?;
                    let head = text[..in_bytes(text, t.get_dst_col())].trim_end();
                    let head = head.strip_suffix('(')?;
                    let head = head.trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == '$');
                    let dot = head.strip_suffix('.')?;
                    Some((t.get_dst_line(), dot.strip_suffix('?').unwrap_or(dot).len() as u32))
                })
        {
            inside
                .entry(found.0)
                .or_default()
                .push((found.1, Inside::BeforeCode, &comment.text));
            continue;
        }
        // On a `_` arm's pattern: after the `default:` above the JS of the
        // arm's first code, past the comments between.
        if comment.of_default {
            let first = (map.get_tokens())
                .filter(|t| t.get_src_line() > comment.line)
                .min_by_key(|t| (t.get_src_line(), t.get_src_col()));
            let clause = first.and_then(|t| {
                let lines = generated_lines.get(..=t.get_dst_line() as usize)?;
                (lines.iter().enumerate().rev())
                    .take_while(|(k, text)| *k == t.get_dst_line() as usize || is_comment_or_default(text))
                    .find(|(_, text)| text.trim_start().starts_with("default:"))
                    .map(|(k, text)| (k as u32, text))
            });
            if let Some((line, text)) = clause {
                let colon = text.find("default:").expect("found") + "default:".len();
                // Before its block's `{`, `default: /* .. */ {`; ending
                // its line where its first statement is on it, the rest on
                // the next, `default: /* .. */\n log.push(row);` (d1).
                let rest = text[colon..].trim_start();
                match (rest.starts_with('{'), rest.is_empty()) {
                    (true, _) => {
                        inside
                            .entry(line)
                            .or_default()
                            .push(((colon + 1) as u32, Inside::BeforeCode, &comment.text))
                    }
                    (false, false) => ends.entry(line).or_default().push((colon as u32, &comment.text)),
                    (false, true) => after.entry(line).or_default().push(&comment.text),
                }
            }
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
        // The nearest before it whose JS is a name, a number or a string;
        // and, of a line comment's end, or a bracket that closes, an
        // array's `]` the map places at its end, `&[""], // path`'s, an
        // argument promoted to a constant (rd525).
        let nearest = |of: &dyn Fn(&str, u32) -> bool| {
            let mut tokens: Vec<_> = (map.get_tokens())
                .filter(|t| t.get_src_line() == comment.line && t.get_src_col() < comment.column)
                .collect();
            tokens.sort_by_key(|t| std::cmp::Reverse(t.get_src_col()));
            tokens
                .into_iter()
                .map(|t| (t.get_dst_line(), t.get_dst_col()))
                .find(|&(line, column)| (generated_lines.get(line as usize)).is_some_and(|text| of(text, column)))
        };
        let before = nearest(&|text, column| word_end(text, column).is_some());
        let closed = || {
            nearest(&|text, column| {
                word_end(text, column).is_some()
                    || text
                        .get(in_bytes(text, column)..)
                        .is_some_and(|rest| rest.starts_with([']', ')', '}']))
            })
        };
        // After an argument's brackets alone, `}, // defaults to true`, of
        // a line no JS may map to: the argument's lines, from where its
        // bracket opened, whose JS it's after (rd615); a bracket's JS of
        // its own line is the nearest before it, first.
        let argument = (comment.opened_on).map(|open| open..comment.line + 1);
        let lines = argument.clone().unwrap_or(comment.line..comment.line + 1);
        let of_lines = || (map.get_tokens()).filter(|t| lines.contains(&t.get_src_line()));
        // Its last JS, where the argument's ends.
        let argument = || {
            argument
                .as_ref()
                .and(of_lines().map(|t| (t.get_dst_line(), t.get_dst_col())).max())
        };
        // Where its JS ends, of a line comment, when a later line's JS
        // follows it on its line: before that, and the `(`, `?`, `:` or
        // `,` it's after.
        let end = (before.or_else(closed).or_else(argument))
            .filter(|_| !comment.inside)
            .and_then(|(line, column)| {
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
        // Of one inside with no such code before it on its line, the code
        // nearest after it there, which it's before.
        let next = || {
            (map.get_tokens())
                .filter(|t| t.get_src_line() == comment.line && t.get_src_col() > comment.column)
                .min_by_key(|t| t.get_src_col())
                .and_then(|t| {
                    let text = generated_lines.get(t.get_dst_line() as usize)?;
                    Some((t.get_dst_line(), in_bytes(text, t.get_dst_col()) as u32))
                })
        };
        match (before, end) {
            (Some((line, column)), _) if comment.inside => {
                inside
                    .entry(line)
                    .or_default()
                    .push((column, Inside::AfterCode, &comment.text))
            }
            (None, _)
                if comment.inside
                    && let Some((line, column)) = next() =>
            {
                inside
                    .entry(line)
                    .or_default()
                    .push((column, Inside::BeforeCode, &comment.text))
            }
            (_, Some((line, end))) => ends.entry(line).or_default().push((end, &comment.text)),
            _ => {
                let first = of_lines().map(|t| (t.get_dst_line(), t.get_dst_col())).min();
                let last = (of_lines().map(|t| t.get_dst_line()).max())
                    .zip(first)
                    .map(|(last, first)| closed_after(first, last, comment.closes, generated_lines));
                if let Some(last) = last {
                    after.entry(last).or_default().push(&comment.text);
                }
            }
        }
    }
    Placed { after, inside, ends }
}

/// Of `last`, the last line of the JS of a comment's line, from `first`,
/// its first JS's line and column, whose code ends closing `closes`
/// brackets: the last of the lines after it that hold only brackets the
/// JS of the line opened and left open, `];` of an array the JS writes an
/// item a line (rd605). Not one of what's around it, the `};` of an object
/// after `o: Cell::new(None), // object cache`, whose `)` no JS has.
fn closed_after(first: (u32, u32), last: u32, closes: u32, generated_lines: &[&str]) -> u32 {
    let (lines, column) = (generated_lines.get(first.0 as usize..=last as usize), first.1);
    let Some(lines) = lines.filter(|_| closes > 0) else {
        return last;
    };
    let text = (lines.iter().enumerate())
        .map(|(k, line)| if k == 0 { &line[in_bytes(line, column)..] } else { line })
        .collect::<Vec<_>>()
        .join("\n");
    let mut open = opened(&text).min(closes);
    let mut at = last;
    while open > 0
        && let Some(next) = generated_lines.get(at as usize + 1)
        && let text = next.trim()
        && !text.is_empty()
        && text.chars().all(|c| ")]};,".contains(c))
    {
        let closing = text.chars().filter(|c| ")]}".contains(*c)).count() as u32;
        if closing > open {
            break;
        }
        open -= closing;
        at += 1;
    }
    at
}

/// How many brackets `code` opens and leaves open, past its strings.
fn opened(code: &str) -> u32 {
    let mut depth: Vec<char> = Vec::new();
    let mut chars = code.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' | '`' => {
                while let Some(d) = chars.next() {
                    match d {
                        '\\' => {
                            chars.next();
                        }
                        d if d == c => break,
                        _ => {}
                    }
                }
            }
            '(' | '[' | '{' => depth.push(c),
            ')' | ']' | '}' => {
                depth.pop();
            }
            _ => {}
        }
    }
    depth.len() as u32
}

/// Whether a line of the JS is a comment or a `default:`, what's between a
/// `default:` and its first code.
fn is_comment_or_default(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with("//") || text.starts_with("/*") || text.starts_with('*') || text.starts_with("default:")
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
    let mut at: Vec<(usize, &str, bool)> = (comments.iter())
        .filter_map(|&(column, inside, text)| match inside {
            Inside::AfterCode => Some((word_end(line, column)?, text, false)),
            Inside::AfterEquals => {
                (line.get(column as usize..)?.starts_with('=')).then_some((column as usize + 1, text, false))
            }
            // Before what's there: the comment, then a space, `(/* c */
            // form`, `/* c */ .r(`, which the formatter writes `/* c */.r(`.
            Inside::BeforeCode => line.get(column as usize..).map(|_| (column as usize, text, true)),
        })
        .collect();
    // From the last, so each column is still where it was.
    at.sort_by_key(|&(end, ..)| std::cmp::Reverse(end));
    let mut moved = Vec::new();
    for (end, text, before) in at {
        let inserted = match before {
            true => format!("{text} "),
            false => format!(" {text}"),
        };
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

/// Where a block comment `c` alone on its line between an `if`'s or a
/// `while`'s condition and its block's `{` goes, as rustfmt writes `if a
/// == 47 /* "/" */ {` too long for its line: right after the condition's
/// last code, as if written there, `=== 47 /* "/" */) {`, as React's
/// ReactFlightPerformanceTrack has it (rd344). What's between it and the
/// code and the `{` may be other comments only; the condition's lines are
/// those of an `if`, an `else if`, a `while` and the `&&` and `||` that go
/// on from one.
pub(super) fn own_line_before_block(
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
    let end = text[..before].trim_end().len();
    let code_line_start = text[..end].rfind('\n').map_or(0, |i| i + 1);
    let condition = text[code_line_start..end].trim_start();
    let opens = ["if ", "} else if ", "while ", "&& ", "|| "];
    if !text[after..].starts_with('{') || !opens.iter().any(|o| condition.starts_with(o)) {
        return None;
    }
    let line = text[..end].matches('\n').count() as u32;
    Some((line, (end - code_line_start) as u32))
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

#[cfg(test)]
mod tests {
    use super::opened_on;

    // A comment after an argument's closing brackets alone is told the
    // line its `if` opened on, past its `else`; one after code isn't
    // (rd615).
    #[test]
    fn opened_on_is_of_closing_brackets_alone() {
        let text = "create(\n    if dev {\n        a\n    } else {\n        b\n    }, // defaults to true\n    false, // find\n)\n";
        let at = |comment: &str| {
            let at = text.find(comment).expect("written");
            (text[..at].rfind('\n').map_or(0, |i| i + 1), at)
        };
        assert_eq!(opened_on(text, at("// defaults")), Some(1));
        assert_eq!(opened_on(text, at("// find")), None);
    }
}
