//! What a declaration carries of the comments written before its item: its
//! doc comment as JSDoc (ADR 0444), and the `//` comments before it, as
//! written (ADR 0196, amended). A port's types carry upstream's Flow types'
//! comments in `//`.

use rustc_hir as hir;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use serde_json::Value;

/// `declaration` with what's written before its item: its `//` comments,
/// then its doc comment as JSDoc, which a TypeScript caller's editor shows:
/// a function's, a type's, a field's.
pub(in crate::lower) fn documented(mut declaration: Value, tcx: TyCtxt<'_>, def_id: DefId) -> Value {
    let jsdoc = jsdoc_of(tcx, def_id);
    let written = leading_comments(tcx, def_id);
    let all: Vec<String> = written.into_iter().chain(jsdoc).collect();
    if !all.is_empty() {
        declaration["docs"] = all.join("\n").into();
    }
    declaration
}

/// `def_id`'s doc comment as JSDoc, `/** .. */`, as `comments::jsdoc`
/// writes it: a declaration's, and a field's in its struct's one literal
/// (ADR 0444, amended). None where it has none.
pub(in crate::lower) fn jsdoc_of(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    use rustc_hir::attrs::AttributeKind;
    let docs: Vec<String> = (rustc_hir::attrs::HasAttrs::get_attrs(def_id, &tcx).iter())
        .filter_map(|attr| match attr {
            // A block's, `/** .. */`, as rustdoc reads it, its stars and
            // its first and last empty lines left out (rd67).
            hir::Attribute::Parsed(AttributeKind::DocComment { kind, comment, .. }) => {
                Some(match kind.comment_kind() {
                    rustc_ast::token::CommentKind::Block => {
                        rustc_ast::util::comments::beautify_doc_string(*comment, rustc_ast::token::CommentKind::Block)
                            .as_str()
                            .trim_end()
                            .to_string()
                    }
                    rustc_ast::token::CommentKind::Line => comment.to_string(),
                })
            }
            _ => None,
        })
        // Each line, an empty `///`'s too, which `lines()` gives none of.
        .flat_map(|doc| doc.split('\n').map(str::to_string).collect::<Vec<_>>())
        .collect();
    let lines: Vec<&str> = docs.iter().map(String::as_str).collect();
    crate::comments::jsdoc(&lines).filter(|_| !lines.is_empty())
}

/// The `//` and `/* */` comments written right before `def_id`'s item, past
/// its attributes and doc comments, each as written, and a blank line where
/// one is written between them or before the item; none of an item
/// nothing but comments is before, whose comments are its file's, a
/// license's (`comments::leading_comments`), nor of one a macro made.
fn leading_comments(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    use rustc_lexer::TokenKind;
    let span = tcx.def_span(def_id.as_local()?);
    if span.from_expansion() || span.is_dummy() {
        return None;
    }
    let file = tcx.sess.source_map().lookup_source_file(span.lo());
    let src = file.src.as_deref()?;
    let before = src.get(..(span.lo() - file.start_pos).0 as usize)?;
    let mut tokens = Vec::new();
    let mut at = 0;
    for token in rustc_lexer::tokenize(before, rustc_lexer::FrontmatterAllowed::Yes) {
        tokens.push((token.kind, &before[at..at + token.len as usize]));
        at += token.len as usize;
    }
    // From the item back: each comment, and `""` for a blank line after it.
    let mut found: Vec<&str> = Vec::new();
    let mut blank = false;
    let mut i = tokens.len();
    loop {
        let Some(next) = i.checked_sub(1) else {
            // Its file's comments, before its first code.
            return None;
        };
        i = next;
        match tokens[i] {
            (TokenKind::Whitespace, text) => blank |= text.matches('\n').count() > 1,
            (
                TokenKind::LineComment { doc_style: None }
                | TokenKind::BlockComment {
                    doc_style: None,
                    terminated: true,
                },
                text,
            ) => {
                // One after code on its line is that code's.
                let start = text.as_ptr() as usize - before.as_ptr() as usize;
                let line = before[..start].rfind('\n').map_or(0, |n| n + 1);
                if !before[line..start].trim().is_empty() {
                    break;
                }
                let text = text.trim_end_matches('\r');
                if !crate::comments::is_one_in_js(text) {
                    return None;
                }
                if blank {
                    found.push("");
                }
                blank = false;
                found.push(text);
            }
            // A doc comment, its JSDoc's.
            (TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }, _) => {}
            // An attribute, `#[..]`: back past its `[`, to its `#`.
            (TokenKind::CloseBracket, _) => {
                let mut depth = 0;
                loop {
                    match tokens[i].0 {
                        TokenKind::CloseBracket => depth += 1,
                        TokenKind::OpenBracket => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                    i = i.checked_sub(1)?;
                }
                i = i.checked_sub(1)?;
                if tokens[i].0 != TokenKind::Pound {
                    break;
                }
            }
            _ => break,
        }
    }
    found.reverse();
    (!found.is_empty()).then(|| found.join("\n"))
}
