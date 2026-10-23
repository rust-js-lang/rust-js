//! How a string literal is written in its source, which its JS writes as it
//! is: one literal; one across lines, a template literal of them; or
//! `concat!`'s pieces, each its own literal joined by `+` (ADR 0641). The
//! one answer of what pieces a string is written as, wherever a literal's
//! value is lowered: in THIR, in MIR's constants, and as a `const`'s value;
//! and of a format string, which `format_args.rs` breaks its template by.

use rustc_ast::token;
use rustc_lexer::{LiteralKind, TokenKind};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LocalDefId;
use rustc_span::{Span, Symbol};

use super::recognition;
use crate::js::{self, Expr, StrPiece};

/// The JS of `text`, the value of the string literal written at `span`, as
/// it's written there: `concat!`'s pieces where it's what `concat!` made of
/// literals, and otherwise the one literal it is.
pub(super) fn string_literal(tcx: TyCtxt<'_>, text: &str, span: Span) -> Expr {
    let pieces = match recognition::from_concat(tcx, span) {
        true => concat_pieces(tcx, text, span),
        false => None,
    };
    Expr::written(pieces.unwrap_or_else(|| {
        let lines = !span.from_expansion()
            && tcx
                .sess
                .source_map()
                .span_to_snippet(span)
                .is_ok_and(|code| written_across_lines(text, &code));
        vec![StrPiece {
            text: text.to_owned(),
            lines,
        }]
    }))
}

/// A `const`'s or a `static`'s string, `const PAGE: &str = r#"<main>..`
/// or one of `concat!`'s, as its literal is written, as one written in
/// place is.
pub(super) fn const_string(tcx: TyCtxt<'_>, def_id: LocalDefId, value: Expr) -> Expr {
    let js::ExprKind::Str(text) = &value.kind else {
        return value;
    };
    match tcx.hir_maybe_body_owned_by(def_id).map(|body| body.value.kind) {
        Some(rustc_hir::ExprKind::Lit(lit)) if matches!(lit.node, rustc_ast::LitKind::Str(..)) => {
            string_literal(tcx, text, lit.span).or_at(value.span)
        }
        _ => value,
    }
}

/// Whether a string literal whose value is `text` and whose code is `code`
/// has its line breaks written as such, not as `\n`: a raw string's always
/// are, and a cooked one's where it spans lines, other than by a `\`
/// ending one, which leaves the break out.
fn written_across_lines(text: &str, code: &str) -> bool {
    text.contains('\n')
        && match code.chars().next() {
            Some('r') => true,
            Some('"') => code.match_indices('\n').any(|(i, _)| !code[..i].ends_with('\\')),
            _ => false,
        }
}

/// How the format string of the macro call at `span`, `format!(..)` or
/// `panic!(..)`, is written, as rustc keeps no span of its own: the call's
/// first string literal, one piece, `format!("..", ..)`'s and `write!(f,
/// "..", ..)`'s; or its first `concat!` of literals, its pieces,
/// `format!(concat!("..", ".."), ..)`'s (ADR 0641). Each piece's text is
/// the format string's, `{}` and all, which the caller checks against the
/// template rustc made of it. `None` where it has neither.
pub(super) fn format_string(tcx: TyCtxt<'_>, span: Span) -> Option<Vec<StrPiece>> {
    let code = tcx.sess.source_map().span_to_snippet(span.source_callsite()).ok()?;
    let tokens = tokens(&code);
    for (i, &(kind, code)) in tokens.iter().enumerate() {
        match kind {
            TokenKind::Literal {
                kind: LiteralKind::Str { .. } | LiteralKind::RawStr { .. },
                ..
            } => return Some(vec![piece(kind, code)?]),
            // To its first closing delimiter, which ends it where its
            // arguments are literals, and otherwise ends what isn't one.
            TokenKind::Ident
                if code == "concat" && tokens.get(i + 1).is_some_and(|(kind, _)| *kind == TokenKind::Bang) =>
            {
                let close = (tokens[i..].iter()).position(|(kind, _)| {
                    matches!(
                        kind,
                        TokenKind::CloseParen | TokenKind::CloseBracket | TokenKind::CloseBrace
                    )
                })?;
                return concat_arguments(&tokens[i..=i + close]);
            }
            _ => {}
        }
    }
    None
}

/// `code`'s tokens, by Rust's lexer, each its kind and its text: not its
/// whitespace or comments.
fn tokens(code: &str) -> Vec<(TokenKind, &str)> {
    let mut at = 0;
    rustc_lexer::tokenize(code, rustc_lexer::FrontmatterAllowed::No)
        .map(|token| {
            let text = &code[at..at + token.len as usize];
            at += token.len as usize;
            (token.kind, text)
        })
        .filter(|(kind, _)| {
            !matches!(
                kind,
                TokenKind::Whitespace | TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }
            )
        })
        .collect()
}

/// The pieces of `concat!(..)`, written at `span`, whose value is `text`:
/// each argument's text, as `concat!` writes it, a string's, a char's, an
/// integer's, a float's or a bool's. `None` where an argument isn't a
/// literal, a macro's value, `stringify!(b)`, which rustc expands first,
/// or where the pieces aren't `text`, which the one literal then is. Of
/// `println!(concat!(..))`, `text` ends with the line break `println!`
/// writes after them, which the last piece then does.
fn concat_pieces(tcx: TyCtxt<'_>, text: &str, span: Span) -> Option<Vec<StrPiece>> {
    let code = tcx.sess.source_map().span_to_snippet(span).ok()?;
    let mut pieces = concat_arguments(&tokens(&code))?;
    let joined: String = pieces.iter().map(|piece| piece.text.as_str()).collect();
    match text.strip_suffix('\n') {
        _ if joined == text => {}
        Some(line) if line == joined => pieces.last_mut()?.text.push('\n'),
        _ => return None,
    }
    Some(pieces)
}

/// The pieces of the call `concat!(..)`, its `tokens`: each argument's text,
/// as `concat!` writes it, where each is a literal.
fn concat_arguments(tokens: &[(TokenKind, &str)]) -> Option<Vec<StrPiece>> {
    // `concat!(..)`, or `std::concat![..]`: its path, then its arguments.
    let bang = tokens.iter().position(|(kind, _)| *kind == TokenKind::Bang)?;
    let path = &tokens[..bang];
    if !path
        .iter()
        .all(|(kind, _)| matches!(kind, TokenKind::Ident | TokenKind::Colon))
    {
        return None;
    }
    let [(open, _), arguments @ .., (close, _)] = &tokens[bang + 1..] else {
        return None;
    };
    if !matches!(
        (open, close),
        (TokenKind::OpenParen, TokenKind::CloseParen)
            | (TokenKind::OpenBracket, TokenKind::CloseBracket)
            | (TokenKind::OpenBrace, TokenKind::CloseBrace)
    ) {
        return None;
    }
    // `concat!("a",)`: a comma after the last.
    let arguments = match arguments {
        [rest @ .., (TokenKind::Comma, _)] => rest,
        _ => arguments,
    };
    let mut pieces = Vec::new();
    for argument in arguments.split(|(kind, _)| *kind == TokenKind::Comma) {
        match argument {
            [(TokenKind::Minus, _), (kind, code)] => pieces.push(StrPiece {
                text: format!("-{}", number(*kind, code)?),
                lines: false,
            }),
            [(kind, code)] => pieces.push(piece(*kind, code)?),
            _ => return None,
        }
    }
    Some(pieces)
}

/// One of `concat!`'s arguments, a literal written as `code`, as `concat!`
/// writes it.
fn piece(kind: TokenKind, code: &str) -> Option<StrPiece> {
    let text = match kind {
        TokenKind::Ident if code == "true" || code == "false" => code.to_owned(),
        TokenKind::Literal {
            kind: LiteralKind::Str { terminated: true },
            suffix_start,
        } if suffix_start as usize == code.len() => value(token::LitKind::Str, &code[1..code.len() - 1], None)?,
        TokenKind::Literal {
            kind: LiteralKind::RawStr { n_hashes: Some(n) },
            suffix_start,
        } if suffix_start as usize == code.len() => {
            let hashes = n as usize;
            value(
                token::LitKind::StrRaw(n),
                &code[2 + hashes..code.len() - 1 - hashes],
                None,
            )?
        }
        TokenKind::Literal {
            kind: LiteralKind::Char { terminated: true },
            suffix_start,
        } if suffix_start as usize == code.len() => value(token::LitKind::Char, &code[1..code.len() - 1], None)?,
        _ => number(kind, code)?,
    };
    let lines = written_across_lines(&text, code);
    Some(StrPiece { text, lines })
}

/// An integer's or a float's literal, written as `code`, as `concat!`
/// writes it: `16` of `0x10`, a float as it's written.
fn number(kind: TokenKind, code: &str) -> Option<String> {
    let TokenKind::Literal { kind, suffix_start } = kind else {
        return None;
    };
    let (digits, suffix) = code.split_at(suffix_start as usize);
    let suffix = (!suffix.is_empty()).then(|| Symbol::intern(suffix));
    match kind {
        LiteralKind::Int { empty_int: false, .. } => value(token::LitKind::Integer, digits, suffix),
        LiteralKind::Float {
            empty_exponent: false, ..
        } => value(token::LitKind::Float, digits, suffix),
        _ => None,
    }
}

/// What `concat!` makes of a literal token of `kind` whose text is
/// `symbol`, by rustc's reading of it.
fn value(kind: token::LitKind, symbol: &str, suffix: Option<Symbol>) -> Option<String> {
    match rustc_ast::LitKind::from_token_lit(token::Lit::new(kind, Symbol::intern(symbol), suffix)).ok()? {
        rustc_ast::LitKind::Str(s, _) | rustc_ast::LitKind::Float(s, _) => Some(s.as_str().to_owned()),
        rustc_ast::LitKind::Char(c) => Some(c.to_string()),
        rustc_ast::LitKind::Int(n, _) => Some(n.get().to_string()),
        _ => None,
    }
}
