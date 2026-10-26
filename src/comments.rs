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
    use super::leading_comments;

    #[test]
    fn a_blank_line_inside_a_block_comment_parts_nothing() {
        let file = "/*\nA.\n\nB.\n */\npub fn f() {}\n";
        assert_eq!(leading_comments(file), "/*\nA.\n\nB.\n */");
        // Between two comments, it parts a license from the code's.
        let file = "// License.\n\n/*\nA.\n\nB.\n */\npub fn f() {}\n";
        assert_eq!(leading_comments(file), "// License.");
    }
}
