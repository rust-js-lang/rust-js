//! What's before a `switch`'s case (ADR 0629): `// Fallthrough` where the
//! one before falls through to it (ADR 0905), and the comments written
//! before the `match` arm it's of, before its labels (ADR 0905, amended).

use super::*;

/// What a case that falls through to the next says, before it, as React
/// writes it, and ESLint's `no-fallthrough` asks (ADR 0905).
pub(super) const FALLTHROUGH: &str = "// Fallthrough";

/// Before each case a case falls through to, `// Fallthrough` (ADR 0905),
/// in `text`, each one of `found`: but where the program's comment says it
/// already, `said`'s.
pub(super) fn fallthroughs(
    module: &Module,
    said: &std::collections::HashSet<u32>,
    text: &mut String,
    found: &mut Vec<oxc_ast::Comment>,
) {
    for at in fallthrough_starts(module).into_iter().filter(|at| !said.contains(at)) {
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

/// Whether `between`, the code between a comment and the start it's
/// before, is a `_` arm's opening, `_ => {`, and its first `if`'s: then
/// the start is its `default`'s, which the comment is before where a case
/// falls through to it (ADR 0905, amended).
pub(super) fn opens_default(between: &str) -> bool {
    let head = between.trim();
    let head = head.strip_suffix("if").map_or(head, str::trim_end);
    head.strip_suffix('{').map(str::trim_end) == Some("_ =>")
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

/// Where each value of a case of more than one starts, of `module`'s
/// `switch`es: a start a comment between a `match` arm's patterns is
/// before, `"mouseup" // TODO: .. | "mouseout"`'s, as React writes it
/// between its `case`s (rd139); and one above the arm, before its first,
/// `// Unlike commitLayoutEffectsOnFiber ..` above `case HostSingleton:`,
/// and a blank line there (rd143), where its first is printed
/// (`Cx::stmt`'s `switch`). Not of a case of one, whose comment above its
/// arm is its body's (ADR 0590), printed at no start of its own.
pub(super) fn pattern_starts(module: &Module) -> Vec<u32> {
    let mut starts = Vec::new();
    each_switch(module, &mut |cases, _| {
        let stacked = cases.iter().filter(|case| case.values.len() > 1);
        let values = stacked.flat_map(|case| case.values.iter());
        starts.extend(values.filter(|v| !v.span.is_none()).map(|v| v.span.lo));
    });
    starts
}

/// Where each case a case falls through to starts, of `module`'s
/// `switch`es: its first value's, which the case is printed at
/// (`fallen_to`), its comment `FALLTHROUGH` before it.
pub(super) fn fallthrough_starts(module: &Module) -> Vec<u32> {
    let mut starts = Vec::new();
    each_switch(module, &mut |cases, default| {
        starts.extend((0..cases.len()).filter_map(|k| fallen_to(cases, k)));
        starts.extend(default_fallen_to(cases, default));
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
/// through to it (ADR 0636, amended): its first statement's start,
/// `FALLTHROUGH` before it.
pub(crate) fn default_fallen_to(cases: &[js::Case], default: Option<&[js::Stmt]>) -> Option<u32> {
    let first = default?.first()?;
    (cases.last()?.falls && !first.span.is_none()).then_some(first.span.lo)
}

/// Where each case the one before falls through to starts, in order: a
/// start a comment right before the arm it's of is before, only its
/// pattern's other constants between, with its `FALLTHROUGH`, as React
/// writes one (ADR 0905, amended). Another case's is its body's, as the
/// scheduler's are (ADR 0590).
pub(super) fn case_starts(module: &Module) -> Vec<u32> {
    let mut starts = fallthrough_starts(module);
    starts.sort_unstable();
    starts
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
