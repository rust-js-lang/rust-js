//! Templates made: a template literal of texts and values, folding what
//! JS would show of a literal into its text; one written across lines,
//! broken where its format string is; and templates in templates spliced
//! into one (ADRs 0066, 0641).

use super::*;

impl Expr {
    pub fn template(texts: Vec<String>, values: Vec<Expr>) -> Expr {
        Expr::template_across(texts, values, Vec::new())
    }

    /// `template`, of values `across` says, in turn, are written across
    /// lines in its source, or not (`TemplateLayout::across`).
    pub fn template_across(texts: Vec<String>, values: Vec<Expr>, across: Vec<bool>) -> Expr {
        debug_assert_eq!(texts.len(), values.len() + 1);
        // `${false}` is `false`, and `${3}` is `3`: a literal the template
        // would show is written in its text, as JS shows it.
        let mut texts = texts.into_iter();
        let mut folded = vec![texts.next().expect("a text before each value")];
        let (mut kept, mut kept_across) = (Vec::new(), Vec::new());
        let across = across.into_iter().chain(std::iter::repeat(false));
        for ((value, text), across) in values.into_iter().zip(texts).zip(across) {
            let shown = match value.kind {
                ExprKind::Bool(b) => Some(b.to_string()),
                ExprKind::Num(n) if n.fract() == 0.0 && n.abs() < 1e21 => Some((n as i128).to_string()),
                _ => None,
            };
            match shown {
                Some(shown) => {
                    let last = folded.last_mut().expect("a text");
                    last.push_str(&shown);
                    last.push_str(&text);
                }
                None => {
                    kept.push(value);
                    kept_across.push(across);
                    folded.push(text);
                }
            }
        }
        // All of it written so, it's a string.
        if kept.is_empty() {
            return Expr::str(folded.pop().expect("a text"));
        }
        let layout = TemplateLayout {
            across: Across::of(kept_across),
            ..TemplateLayout::default()
        };
        Expr::new(ExprKind::Template(folded, kept, layout))
    }

    /// A template written across `lines`: its line breaks written as they are.
    pub fn with_lines(self, lines: bool) -> Expr {
        match self.kind {
            ExprKind::Template(texts, values, layout) => Expr::new(ExprKind::Template(
                texts,
                values,
                TemplateLayout {
                    lines,
                    breaks: Vec::new(),
                    across: layout.across,
                },
            )),
            _ => self,
        }
    }

    /// A template whose format string is written as `pieces`, `concat!`'s,
    /// each its texts and values, as `template` takes them, and whether
    /// it's written across lines: one template, broken where they are
    /// (ADR 0641). Of no values, the pieces are a string's. Each piece's
    /// values are `across` lines, in turn, or not (`template_across`).
    pub fn template_pieces(pieces: Vec<TemplatePiece>) -> Expr {
        let (mut texts, mut values) = (vec![String::new()], Vec::new());
        let mut layout = TemplateLayout::default();
        let mut strings = Vec::new();
        let mut across = Vec::new();
        for (i, (piece_texts, piece_values, piece_across, lines)) in pieces.into_iter().enumerate() {
            let (piece_texts, piece_values) = match Expr::template_across(piece_texts, piece_values, piece_across).kind
            {
                ExprKind::Str(text) => (vec![text], Vec::new()),
                ExprKind::Template(piece_texts, piece_values, piece) => {
                    across.extend((0..piece_values.len()).map(|i| piece.across.at(i)));
                    (piece_texts, piece_values)
                }
                _ => unreachable!("a template is a string or a template"),
            };
            let last = texts.last_mut().expect("a text");
            match i {
                0 => layout.lines = lines,
                _ => layout.breaks.push(TemplateBreak {
                    text: values.len(),
                    at: last.len(),
                    lines,
                }),
            }
            strings.push(StrPiece {
                text: piece_texts.concat(),
                lines,
                span: Span::NONE,
            });
            let mut piece_texts = piece_texts.into_iter();
            last.push_str(&piece_texts.next().expect("a text"));
            values.extend(piece_values);
            texts.extend(piece_texts);
        }
        layout.across = Across::of(across);
        match values.is_empty() {
            true => Expr::written(strings),
            false => Expr::new(ExprKind::Template(texts, values, layout)),
        }
    }

    /// A template's value that's a template itself, a function's body put
    /// in place of its call, `${`(${a}, ${b})`}`, or a value's text in one,
    /// `${`${x}`}`: its text and values in the outer's, `(${a}, ${b})`, as
    /// a template makes each value's text the same either way. Of an inner
    /// of one piece on one line, and not of one with text into a piece
    /// written across lines, whose line breaks are as they are where the
    /// inner's are `\n`; one of values alone has none (ADR 0066).
    pub fn splice_templates(&mut self) {
        let ExprKind::Template(texts, values, layout) = &mut self.kind else {
            return;
        };
        let outer_lines = layout.lines || layout.breaks.iter().any(|at| at.lines);
        let splices = |v: &Expr| {
            matches!(&v.kind, ExprKind::Template(inner_texts, _, inner)
                if inner.is_plain() && (!outer_lines || inner_texts.iter().all(String::is_empty)))
        };
        if !values.iter().any(splices) {
            return;
        }
        // Where each of the outer's texts is now, its index and how far into
        // it: so `concat!`'s pieces break where they did (ADR 0641).
        let mut moved = vec![(0, 0)];
        let (mut spliced_texts, mut spliced_values) = (vec![texts[0].clone()], Vec::new());
        // Each value's being written across lines, an inner's its own.
        let mut spliced_across = Vec::new();
        let outer_across = std::mem::take(&mut layout.across);
        for (i, (value, after)) in std::mem::take(values).into_iter().zip(texts.iter().skip(1)).enumerate() {
            let spliced = splices(&value);
            match value.kind {
                ExprKind::Template(inner_texts, inner_values, inner_layout) if spliced => {
                    let mut inner = inner_texts.into_iter();
                    spliced_texts
                        .last_mut()
                        .expect("a text")
                        .push_str(&inner.next().unwrap_or_default());
                    for (j, (inner_value, inner_text)) in inner_values.into_iter().zip(inner).enumerate() {
                        spliced_values.push(inner_value);
                        spliced_across.push(inner_layout.across.at(j));
                        spliced_texts.push(inner_text);
                    }
                }
                _ => {
                    spliced_values.push(value);
                    spliced_across.push(outer_across.at(i));
                    spliced_texts.push(String::new());
                }
            }
            let last = spliced_texts.last_mut().expect("a text");
            moved.push((spliced_values.len(), last.len()));
            last.push_str(after);
        }
        for at in &mut layout.breaks {
            let (text, shift) = moved[at.text];
            (at.text, at.at) = (text, shift + at.at);
        }
        *texts = spliced_texts;
        *values = spliced_values;
        layout.across = Across::of(spliced_across);
    }
}
