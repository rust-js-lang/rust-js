//! What's known of a statement, kept on it, and a statement rebuilt in its
//! place keeping it (#1681 change 3, M4, ADR 1167).

use super::*;

/// What's known of a statement (`Stmt::facts`, #1681 change 3, M4): its
/// facts of `Known`'s one catalog, those of a statement alone
/// (`Known::of_statement`), which `--without-known` forgets and
/// `--knowns-found` counts as any fact.
///
/// And of a write, `x = v`, what MIR said of whether what it gives may be
/// `undefined` where it was lowered (`written`): what a pass that learns
/// the place's storage later, a handle's (`tidy::values::handles`,
/// `js::applied`), stores it by, set only by `Stmt::written` and kept only
/// while its value is (#2112 S4b).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct StmtFacts {
    facts: Facts,
    written: Undefined,
}

impl Stmt {
    /// A statement of `kind` in its place, rebuilt of it: its span and
    /// what's known of it, as an `if` made again of the same test keeps
    /// what the program wrote of it (#1681 change 3, M4).
    /// What MIR said of what a write gives (`StmtFacts::written`) is kept
    /// only where its value is: another value is told nothing of it.
    pub fn rebuilt(&self, kind: StmtKind) -> Stmt {
        let kept = matches!((&self.kind, &kind), (StmtKind::Assign(_, was), StmtKind::Assign(_, is)) if was == is);
        let written = match kept {
            true => self.facts.written,
            false => Undefined::ByForm,
        };
        Stmt {
            kind,
            span: self.span,
            facts: StmtFacts { written, ..self.facts },
        }
    }

    /// A write, `kind`, an assignment or what writes through a place, told
    /// MIR's answer of whether what it gives may be `undefined`, `given`,
    /// which a nullable place it turns out to be stores it by (#2112 S4b).
    pub fn written(kind: StmtKind, span: Span, given: Undefined) -> Stmt {
        let mut stmt = kind.at(span);
        stmt.facts.written = given;
        stmt
    }

    /// It, known `known` too.
    pub fn knowing(mut self, known: Known) -> Stmt {
        self.facts = self.facts.with(known);
        self
    }
}

impl StmtFacts {
    /// Whether it says `known`, a statement's fact.
    pub fn has(self, known: Known) -> bool {
        debug_assert!(known.of_statement(), "{known:?} is no statement's fact");
        self.facts.has(known)
    }

    /// What MIR said of whether what a write gives may be `undefined`,
    /// `ByForm` where it said nothing (`Stmt::written`).
    pub fn written(self) -> Undefined {
        self.written
    }

    /// It and `known`, a statement's fact.
    pub fn with(self, known: Known) -> StmtFacts {
        debug_assert!(known.of_statement(), "{known:?} is no statement's fact");
        StmtFacts {
            facts: self.facts.with(known),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A statement rebuilt in its place keeps its span and what's known of
    // it; one made anew knows nothing (ADR 1167).
    #[test]
    fn a_statement_rebuilt_keeps_its_facts() {
        let span = Span { lo: 3, hi: 9 };
        let written = StmtKind::If(Expr::bool(true), Vec::new(), None)
            .at(span)
            .knowing(Known::Else);
        let rebuilt = written.rebuilt(StmtKind::If(Expr::bool(false), Vec::new(), Some(Vec::new())));
        assert!(rebuilt.facts.has(Known::Else));
        assert!(rebuilt.span == span);
        assert!(!StmtKind::Break(None).at(span).facts.has(Known::Else));
    }

    // What MIR said of a write's value is kept where its value is, and
    // forgotten where another value is written (#2112 S4b).
    #[test]
    fn a_write_rebuilt_of_another_value_forgets_its_answer() {
        let span = Span { lo: 3, hi: 9 };
        let target = Expr::var(Name::declared("x".into()));
        let value = Expr::var(Name::declared("v".into()));
        let written = Stmt::written(StmtKind::Assign(target.clone(), value.clone()), span, Undefined::Never);
        assert!(written.rebuilt(StmtKind::Assign(target.clone(), value)).facts.written() == Undefined::Never);
        let other = Expr::var(Name::declared("w".into()));
        assert!(written.rebuilt(StmtKind::Assign(target, other)).facts.written() == Undefined::ByForm);
    }
}
