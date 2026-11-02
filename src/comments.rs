//! Rust's comments as JS can hold them (ADR 0383, ADR 0444): which a file
//! starts with, its JS's header's, and which JS reads as Rust does.

/// The comments a Rust file starts with, `//` and `/* */`, a license's, up
/// to its first doc comment, attribute or item: what its JS starts with too
/// (ADR 0383). A doc comment is its item's or its module's, `///`, `//!`,
/// `/**` and `/*!`, but `////`, `/***` and `/**/` are plain; one that isn't
/// one comment in JS (`is_one_in_js`) ends them.
pub fn leading_comments(text: &str) -> &str {
    let mut end = 0;
    let mut rest = text;
    // Where each comment ends, and the next starts: the gaps between them.
    let mut gaps: Vec<(usize, usize)> = Vec::new();
    loop {
        let trimmed = rest.trim_start();
        let at = text.len() - trimmed.len();
        let doc =
            |marker: &str, plain: &[&str]| trimmed.starts_with(marker) && !plain.iter().any(|p| trimmed.starts_with(p));
        let length = if trimmed.starts_with("//") {
            if doc("///", &["////"]) || trimmed.starts_with("//!") {
                break;
            }
            let line = trimmed.find('\n').unwrap_or(trimmed.len());
            if !is_one_in_js(trimmed[..line].strip_suffix('\r').unwrap_or(&trimmed[..line])) {
                break;
            }
            line
        } else if trimmed.starts_with("/*") {
            if doc("/**", &["/***", "/**/"]) || trimmed.starts_with("/*!") {
                break;
            }
            let Some(close) = trimmed.find("*/") else {
                break;
            };
            if !is_one_in_js(&trimmed[..close + 2]) {
                break;
            }
            close + 2
        } else {
            break;
        };
        if end > 0 {
            gaps.push((end, at));
        }
        end = at + length;
        rest = &text[end..];
    }
    // A run right above the code, no blank line between, is that code's
    // (ADR 0444), where a blank line between two comments parts it from the
    // rest (ADR 0383, amended): a license, then what the first item's
    // comments say. Not one inside a block comment, which is one comment.
    let blank = |gap: &str| gap.matches('\n').count() > 1;
    if text[end..].starts_with('\n')
        && !text[end..].starts_with("\n\n")
        && !text[end..].starts_with("\n\r\n")
        && let Some(&(before, _)) = gaps.iter().rev().find(|&&(from, to)| blank(&text[from..to]))
    {
        return &text[..before];
    }
    &text[..end]
}

/// A Rust comment as JS writes it: an inner doc comment, a module's or a
/// block's, `//!` or `/*!`, is plain in JS, which has no doc comment of
/// what it's in, `//` or `/*` (ADR 0444, amended); every other as written.
pub fn in_js(comment: &str) -> String {
    match (comment.strip_prefix("//!"), comment.strip_prefix("/*!")) {
        (Some(rest), _) => format!("//{rest}"),
        (_, Some(rest)) => format!("/*{rest}"),
        _ => comment.to_string(),
    }
}

/// Whether a Rust comment, as written, without a CRLF's `\r`, is one comment
/// in JS too, and so can be written there: JS ends a block comment at its
/// first `*/`, where Rust nests them, and a line comment at a carriage
/// return, U+2028 or U+2029, where Rust's goes on, so its rest would be code.
pub fn is_one_in_js(comment: &str) -> bool {
    match comment.strip_prefix("/*") {
        Some(inner) => !inner.contains("/*"),
        None => !comment.contains(['\r', '\u{2028}', '\u{2029}']),
    }
}

/// Of each of `comments`, where they are in `text`, in order, whether the JS
/// leaves it out: one that begins `SAFETY:`, `// SAFETY: ..` above an
/// `unsafe` block, as clippy asks of one, says why Rust's `unsafe` holds,
/// which JS has none of (ADR 0444, amended, the user's decision); and so
/// does each line comment on the lines right below one on lines of its
/// own, the rest of what it says. A doc comment, `/// SAFETY:`, is its item's, kept.
pub fn left_out(text: &str, comments: &[(usize, usize)]) -> Vec<bool> {
    let mut out = Vec::with_capacity(comments.len());
    let mut run_end = None;
    for &(lo, hi) in comments {
        let written = &text[lo..hi];
        let said = (written.strip_prefix("//").or_else(|| written.strip_prefix("/*")))
            .is_some_and(|rest| rest.trim_start().starts_with("SAFETY:"));
        // The next line's, nothing but its indentation before it.
        let continued = run_end.is_some_and(|end: usize| {
            let gap = &text[end..lo];
            written.starts_with("//")
                && !written.starts_with("///")
                && !written.starts_with("//!")
                && gap.matches('\n').count() == 1
                && gap.trim().is_empty()
        });
        let out_of_js = said || continued;
        // Of one on lines of its own, not one after code on its line.
        let own_line = text[..lo]
            .rsplit('\n')
            .next()
            .is_some_and(|line| line.trim().is_empty());
        run_end = (out_of_js && own_line && written.starts_with("//")).then_some(hi);
        out.push(out_of_js);
    }
    out
}

/// A doc comment's lines, each as `///` or `#[doc]` writes it, its first
/// space but not its text's: their JSDoc, `/** one */` or one line each
/// (ADR 0444); none where one has the `*/` that would end it.
pub fn jsdoc(lines: &[&str]) -> Option<String> {
    if lines.iter().any(|line| line.contains("*/")) {
        return None;
    }
    let lines: Vec<&str> = (lines.iter()).map(|l| l.strip_prefix(' ').unwrap_or(l)).collect();
    Some(match lines.as_slice() {
        [one] => format!("/** {one} */"),
        _ => {
            let lines: Vec<String> = (lines.iter())
                .map(|line| format!(" * {line}").trim_end().to_string())
                .collect();
            format!("/**\n{}\n */", lines.join("\n"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{leading_comments, left_out};

    #[test]
    fn a_safety_comment_and_its_lines_below_are_left_out() {
        let text =
            "// Before.\n// SAFETY: a\n// b.\n\n// After.\nlet x = 1; // SAFETY: c.\n// Not its.\n/* SAFETY: d */";
        let spans: Vec<(usize, usize)> = (text.match_indices("//").map(|(lo, _)| lo))
            .chain(text.find("/*"))
            .map(|lo| {
                let end = match text[lo..].starts_with("/*") {
                    true => text[lo..].find("*/").unwrap() + 2,
                    false => text[lo..].find('\n').unwrap_or(text.len() - lo),
                };
                (lo, lo + end)
            })
            .collect();
        assert_eq!(left_out(text, &spans), [false, true, true, false, true, false, true]);
    }

    #[test]
    fn a_blank_line_inside_a_block_comment_parts_nothing() {
        let file = "/*\nA.\n\nB.\n */\npub fn f() {}\n";
        assert_eq!(leading_comments(file), "/*\nA.\n\nB.\n */");
        // Between two comments, it parts a license from the code's.
        let file = "// License.\n\n/*\nA.\n\nB.\n */\npub fn f() {}\n";
        assert_eq!(leading_comments(file), "// License.");
    }
}

/// The run of `///` doc comment lines `own` starts with, and each one's text
/// past its `///`: what's one JSDoc before an item, or a re-export (`jsdoc`).
pub fn doc_lines<'s>(own: &[crate::program::Comment], text: &'s str) -> (usize, Vec<&'s str>) {
    let run = (own.iter())
        .take_while(|c| c.doc && text[c.lo as usize..].starts_with("///"))
        .count();
    let lines = (own[..run].iter())
        .map(|c| text[c.lo as usize..c.hi as usize].trim_start_matches("///"))
        .collect();
    (run, lines)
}
