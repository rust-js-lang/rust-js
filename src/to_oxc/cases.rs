//! What's before a `switch`'s case (ADR 0629): `// falls through` where the
//! one before falls through to it (ADR 0905), and the comments written
//! before the `match` arm it's of, before its labels (ADR 0905, amended).

use super::*;

/// What a case that falls through to the next says, before it, as React
/// writes it, and ESLint's `no-fallthrough` asks (ADR 0905).
pub(super) const FALLTHROUGH: &str = "// falls through";

/// Before each case a case falls through to, `// falls through` (ADR 0905),
/// in `text`, each one of `found`, `source` the program: but where the
/// program's comment says it already, `said`'s.
pub(super) fn fallthroughs(
    (module, source): (&Module, &str),
    said: &std::collections::HashSet<u32>,
    text: &mut String,
    found: &mut Vec<oxc_ast::Comment>,
) {
    for at in fallthrough_starts(module, source)
        .into_iter()
        .filter(|at| !said.contains(at))
    {
        let lo = text.len() as u32;
        text.push_str(FALLTHROUGH);
        let mut made = oxc_ast::Comment::new(lo, text.len() as u32, oxc_ast::CommentKind::Line);
        made.attached_to = at;
        made.position = oxc_ast::CommentPosition::Leading;
        made.set_preceded_by_newline(true);
        made.set_followed_by_newline(true);
        found.push(made);
    }
}

/// Where a clause whose arm opens a block, of `body`, is printed: just past
/// its `{` (`opened_at`). Not of an arm of only comments, `_ => { //
/// Unknown event. }`, which are the clause's own where it ends, before its
/// `break;`.
pub(crate) fn clause_start(source: &str, body: &[js::Stmt]) -> Option<u32> {
    if matches!(body.first().map(|s| &s.kind), Some(StmtKind::Comments)) {
        return None;
    }
    opened_at(source, default_start(Some(body))?)
}

/// Where the arm whose first statement starts at `lo` opens its block,
/// `_ => {`, `"meta" => {`, of one pattern on its line: just past its `{`,
/// past which what's on lines of their own before `lo` is comments only.
pub(crate) fn opened_at(source: &str, lo: u32) -> Option<u32> {
    let before = source.get(..lo as usize)?;
    // Of a first statement whose start is its value's, `let mut tail_node =
    // render_state.tail.borrow().clone();`'s, from its `let`, the line it's
    // written on (rd532).
    let mut end = let_start(before).unwrap_or(before.len());
    // Back past the block's own comments, each on a line of its own.
    loop {
        let code = before[..end].trim_end();
        let line = code.rfind('\n').map_or(0, |i| i + 1);
        match code[line..].trim_start().starts_with("//") {
            true => end = line,
            false => {
                end = code.len();
                break;
            }
        }
    }
    let code = &before[..end];
    let line = code.rfind('\n').map_or(0, |i| i + 1);
    // Of a first statement that's an `if` whose span starts at its test,
    // `if __DEV__ && row.is_empty() {`: the arm's opening the line before
    // its `if`, and the clause where that statement starts, as a case falls
    // through to it there (rd525).
    if code[line..].trim() == "if"
        && line > 0
        && opens_default(&code[code[..line - 1].rfind('\n').map_or(0, |i| i + 1)..])
    {
        return Some(lo);
    }
    opens_default(&code[line..]).then_some(end as u32)
}

/// Where the line of the `let` that `before` ends with starts, `let mut n
/// =`, the value its statement starts at next.
fn let_start(before: &str) -> Option<usize> {
    let code = before.trim_end();
    let line = code.rfind('\n').map_or(0, |i| i + 1);
    let head = code[line..].trim_start();
    (head.starts_with("let ") && head.ends_with('=')).then_some(line)
}

/// Whether `between`, the code between a comment and the start it's
/// before, is a `_` arm's opening, `_ => {`, and its first `if`'s: then
/// the start is its `default`'s, which the comment is before where a case
/// falls through to it (ADR 0905, amended); or an arm's of one pattern on
/// its line, `"meta" => {`, its case's (rd166); a block comment on the
/// pattern, `67 /* "C" */ => {`, as if it weren't there (rd525).
pub(super) fn opens_default(between: &str) -> bool {
    // Past a block comment on its pattern, `_ /* "{" "[" */ => {` (rd525).
    let code = uncommented(between);
    let head = code.trim();
    let head = head.strip_suffix("if").map_or(head, str::trim_end);
    head.strip_suffix('{')
        .map(str::trim_end)
        .and_then(|head| head.strip_suffix("=>"))
        .is_some_and(|pattern| !pattern.trim().is_empty() && !pattern.contains(['\n', ';', '{', '}', '|']))
}

/// `text` with each block comment in it a space, so a pattern's, `125 /*
/// "}" */ =>`, reads as if it weren't there (rd525).
fn uncommented(text: &str) -> String {
    let mut code = text.to_string();
    while let Some(open) = code.find("/*")
        && let Some(close) = code[open..].find("*/")
    {
        code.replace_range(open..open + close + 2, " ");
    }
    code
}

/// Whether a comment the program wrote says a case falls through, as
/// ESLint's `no-fallthrough` reads one, `/falls?\s?through/i`: `// falls
/// through`, `// Fall through`; then it's said, and `FALLTHROUGH` isn't
/// written beside it (ADR 0905, amended).
pub(super) fn says_fallthrough(comment: &str) -> bool {
    let text = comment.to_lowercase();
    ["fall through", "falls through", "fallthrough", "fallsthrough"]
        .iter()
        .any(|said| text.contains(said))
}

/// Where each value of a case printed where it's written starts
/// (`at_its_value`), of `module`'s `switch`es, `source` the program: a
/// start a comment between a `match` arm's patterns is before, `"mouseup"
/// // TODO: .. | "mouseout"`'s, as React writes it between its `case`s
/// (rd139); and one above the arm, before its first, `// Unlike
/// commitLayoutEffectsOnFiber ..` above `case HostSingleton:`, and a blank
/// line there (rd143), where its first is printed (`Cx::stmt`'s
/// `switch`). Not of a case of one value whose arm is a value, `IMMEDIATE
/// => -1.0`, whose comment above it is its body's (ADR 0590), printed at
/// no start of its own.
pub(super) fn pattern_starts(module: &Module, source: &str) -> Vec<u32> {
    let mut starts = Vec::new();
    each_switch(module, &mut |cases, _| {
        let stacked = cases.iter().filter(|case| at_its_value(case, source));
        let values = stacked.flat_map(|case| case.values.iter());
        starts.extend(values.filter(|v| !v.span.is_none()).map(|v| v.span.lo));
    });
    starts
}

/// Whether `case` is printed where its first value is written, in
/// `source`, so a comment above its arm is above its `case`, as upstream
/// writes one (rd619, the user's decision): one of more than one value
/// (rd139, rd143); and of one whose arm opens a block, `"mousedown" => {`,
/// `"value" => {}`'s that does nothing too, `case "value": break;`
/// (rd136), or is guarded and does nothing, `"selectionchange" if skip =>
/// {}` (`opens_or_guards`).
/// Not of one whose arm is a value, `IMMEDIATE => -1.0`, whose comment
/// above it is its body's, there being no other place to write it (ADR
/// 0590).
pub(crate) fn at_its_value(case: &js::Case, source: &str) -> bool {
    if case.values.is_empty() {
        return false;
    }
    case.values.len() > 1 || written_apart(case, source)
}

/// Whether `case`'s arm, in `source`, where its first value is, opens a
/// block or is guarded and does nothing (`opens_or_guards`): what a
/// comment above it is above, not its body, as upstream writes one
/// (rd619). An arm of only comments, `3 => { // .. }`, too: they're its
/// clause's own, before the `break;` every clause that goes on ends with
/// (ADR 0629, amended).
fn written_apart(case: &js::Case, source: &str) -> bool {
    (case.values.first()).is_some_and(|first| !first.span.is_none() && opens_or_guards(source, first.span.hi))
}

/// Whether the arm whose pattern ends at `hi` in `source` opens a block,
/// `"mousedown" => {`, or is guarded and does nothing, `"selectionchange"
/// if skip => {}`, its case's body only its guard's `if`: what's between
/// the pattern and its `=>`, past the value's own `)` of `Some("a")`, and
/// after it. Not a guarded arm that does something, `"keypress" if
/// char_code(event) == 0 => return None`, whose comment above it is its
/// body's, before the guard's `if`, as upstream writes React DOM's
/// SimpleEventPlugin's and completeWork's.
fn opens_or_guards(source: &str, hi: u32) -> bool {
    let Some(rest) = source.get(hi as usize..) else {
        return false;
    };
    let Some(arrow) = rest.find("=>") else {
        return false;
    };
    // Past a block comment on its pattern, `125 /* "}" */ => {` (rd525).
    let (head, body) = (uncommented(&rest[..arrow]), rest[arrow + 2..].trim_start());
    let pattern = head.split(['{', '}', ';']).next().unwrap_or_default();
    match pattern.split_whitespace().any(|word| word == "if") {
        true => body
            .strip_prefix('{')
            .is_some_and(|inside| inside.trim_start().starts_with('}')),
        false => !head.contains(['{', '}', ';']) && body.starts_with('{'),
    }
}

/// Where each case a case falls through to starts, of `module`'s
/// `switch`es: its first value's, which the case is printed at
/// (`fallen_to`), its comment `FALLTHROUGH` before it.
pub(super) fn fallthrough_starts(module: &Module, source: &str) -> Vec<u32> {
    let mut starts = Vec::new();
    each_switch(module, &mut |cases, default| {
        starts.extend((0..cases.len()).filter_map(|k| fallen_to(cases, k)));
        starts.extend(default_fallen_to(source, cases, default));
    });
    starts
}

/// Where the case `cases[k]` is printed, where the one before falls
/// through to it: its first value's start, `FALLTHROUGH` before it.
pub(crate) fn fallen_to(cases: &[js::Case], k: usize) -> Option<u32> {
    let before = cases.get(k.checked_sub(1)?)?;
    let first = cases.get(k)?.values.first()?;
    (before.falls && !first.span.is_none()).then_some(first.span.lo)
}

/// Where the `default` is printed, `default`, where the last case falls
/// through to it (ADR 0636, amended), `source` the program: of a `_` arm
/// that opens a block, `_ => {`, just past its `{` (`clause_start`), as
/// where none falls to it, so a comment the program wrote above the arm is
/// before it, its `// Fallthrough` said there (rd638); else its first
/// statement's start. `FALLTHROUGH` before it where none is.
pub(crate) fn default_fallen_to(source: &str, cases: &[js::Case], default: Option<&[js::Stmt]>) -> Option<u32> {
    let default = default?;
    let first = default.first()?;
    let fallen = cases.last()?.falls && !first.span.is_none();
    fallen.then(|| clause_start(source, default).unwrap_or(first.span.lo))
}

/// Where each case the one before falls through to starts, in order: a
/// start a comment right before the arm it's of is before, only its
/// pattern's other constants between, with its `FALLTHROUGH`, as React
/// writes one (ADR 0905, amended). Another case's is its body's, as the
/// scheduler's are (ADR 0590).
pub(super) fn case_starts(module: &Module, source: &str) -> Vec<u32> {
    let mut starts = fallthrough_starts(module, source);
    // And each `default`'s, a comment above whose `_` arm is before it, as
    // above a case of the labels a `_` stands for, React's `// The default
    // is now forwards.` above `case 'forwards': default:` (rd214).
    // Where its arm opens a block, just past its `{`: what's in the block is
    // its body's.
    // And each case's whose arm opens a block, `"mousedown" => {`, or is
    // guarded, `"selectionchange" if skip => {}`, where its first value is,
    // a comment above which is before its `case`, not its body's first
    // statement on the guard's line (rd166, rd619).
    each_switch(module, &mut |cases, default| {
        starts.extend(default.and_then(|default| clause_start(source, default)));
        let apart = cases.iter().filter(|case| written_apart(case, source));
        starts.extend(apart.map(|case| case.values[0].span.lo));
    });
    starts.sort_unstable();
    starts.dedup();
    starts
}

/// Where a `switch`'s `default` is printed: its first statement's start.
pub(crate) fn default_start(default: Option<&[js::Stmt]>) -> Option<u32> {
    let first = default?.first()?;
    (!first.span.is_none()).then_some(first.span.lo)
}

/// What's asked of each `switch`: its cases, and its `default`.
type EachSwitch<'f> = dyn FnMut(&[js::Case], Option<&[js::Stmt]>) + 'f;

/// Each `switch` of `module`'s bodies, its cases and its `default`.
fn each_switch(module: &Module, f: &mut EachSwitch<'_>) {
    for mut body in bodies(module) {
        js::each_block_mut(&mut body, &mut |stmts| {
            for stmt in stmts.iter() {
                if let StmtKind::Switch { cases, default, .. } = &stmt.kind {
                    f(cases, default.as_deref());
                }
            }
        });
    }
}
