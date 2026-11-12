//! A line formatted into stdout where formatting may print (#1490, ADR
//! 1130): Rust's stdout is a `LineWriter`, so a print a writer makes as it
//! formats lands inside the line formatted so far, and a writer's text
//! written before its print comes before it. Where `analysis::prints` says
//! a writer may print, `println!` writes each piece through stdout's line
//! as it's made (`$printing`, `$out`), the writer writes its text there as
//! it writes it (`$sink()`), a print of what may run meanwhile is written
//! through the line too, and a writer's text made then for anything else,
//! `format!`'s, is made apart from it (`$formatting`). Elsewhere a line is
//! `console.log`'s and a writer gives back its string, as before.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// May the body being lowered run while a line is formatted?
    pub(in crate::lower) fn runs_while_formatting(&self) -> bool {
        self.krate.prints.runs_while_formatting(self.item)
    }

    /// May showing a value of `ty` print as it's formatted?
    pub(in crate::lower) fn shown_may_print(&self, ty: Ty<'tcx>) -> bool {
        self.krate.prints.shown_may_print(ty)
    }

    /// Does `writer`, a writer of the crate's, write its text through the
    /// line it formats into, as it may print?
    pub(in crate::lower) fn writes_through_the_line(&self, writer: DefId) -> bool {
        self.krate.prints.writer_prints(writer)
    }

    /// `text`, a `format_args!`'s whose formatting may print, made for
    /// anything but stdout's line: as it is, or, where a line may be
    /// formatted meanwhile, apart from it, `$formatting(() => text)`.
    pub(in crate::lower) fn made_apart_from_the_line(&self, text: Expr) -> Expr {
        match self.runs_while_formatting() {
            true => Expr::call_helper(
                "$formatting",
                vec![Expr::arrow(
                    Vec::new(),
                    vec![StmtKind::Return(Some(text)).at(js::Span::NONE)],
                )],
            ),
            false => text,
        }
    }

    /// `print!`'s or `println!`'s `text`, whose formatting may print: each
    /// piece written through stdout's line as it's made, `$printing(() => {
    /// $out("a "); $out(noisyDisplay_fmt(n)); $out(" b\n"); })`. Of what
    /// may `fails`, in a `try`, whose `fmt::Error` is std's panic, what was
    /// written before it in the line (ADR 0187).
    pub(in crate::lower) fn printed_through_the_line(&mut self, text: Expr, fails: bool, js_span: js::Span) -> Expr {
        let mut body: Vec<Stmt> = (line_parts(text).into_iter())
            .map(|part| StmtKind::Expr(Expr::call_helper("$out", vec![part])).at(js_span))
            .collect();
        if fails {
            let caught = self.fresh("error");
            let failed = Expr::call_helper(
                "$fmtPrintFailed",
                vec![Expr::var(&caught), Expr::str(""), Expr::bool(false)],
            );
            body = vec![StmtKind::TryCatch(body, Some(caught), vec![StmtKind::Expr(failed).at(js_span)]).at(js_span)];
        }
        Expr::call_helper("$printing", vec![Expr::arrow(Vec::new(), body)])
    }

    /// A print to stdout of the body being lowered, `text`, where it may
    /// run while a line is formatted: through the line, `$out(text)`, so it
    /// lands inside it. `None` elsewhere.
    pub(in crate::lower) fn print_through_the_line(&self, text: &Expr) -> Option<Expr> {
        self.runs_while_formatting()
            .then(|| Expr::call_helper("$out", vec![text.clone()]))
    }
}

/// The pieces of `written`, a template, each piece of text and each value
/// shown, in order, each written apart; another, as it is.
pub(super) fn line_parts(written: Expr) -> Vec<Expr> {
    let js::ExprKind::Template(texts, values, _) = &written.kind else {
        return vec![written];
    };
    let mut parts = Vec::new();
    let mut values = values.iter().cloned();
    for text in texts {
        if !text.is_empty() {
            parts.push(Expr::str(text));
        }
        parts.extend(values.next());
    }
    parts
}

/// A writer's text as it writes it, `written`, to `target`, the sink it
/// writes through (`$sink()`): `f.write(x)` of each piece, or of it whole.
pub(in crate::lower) fn write_to_sink(
    target: &Expr,
    written: Expr,
    pieces: bool,
    js_span: js::Span,
    out: &mut Vec<Stmt>,
) {
    let parts = match pieces {
        true => line_parts(written),
        false => vec![written],
    };
    for part in parts {
        out.push(StmtKind::Expr(Expr::call(Expr::member(target.clone(), "write"), vec![part])).at(js_span));
    }
}
