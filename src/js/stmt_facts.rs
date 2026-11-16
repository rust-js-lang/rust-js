//! What's known of a statement, kept on it, and a statement rebuilt in its
//! place keeping it (#1681 change 3, M4, ADR 1167).

use super::*;

/// What's known of a statement (`Stmt::facts`, #1681 change 3, M4): its
/// facts of `Known`'s one catalog, those of a statement alone
/// (`Known::of_statement`), which `--without-known` forgets and
/// `--knowns-found` counts as any fact.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct StmtFacts(Facts);

impl Stmt {
    /// A statement of `kind` in its place, rebuilt of it: its span and
    /// what's known of it, as an `if` made again of the same test keeps
    /// what the program wrote of it (#1681 change 3, M4).
    pub fn rebuilt(&self, kind: StmtKind) -> Stmt {
        Stmt {
            kind,
            span: self.span,
            facts: self.facts,
        }
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
        self.0.has(known)
    }

    /// It and `known`, a statement's fact.
    pub fn with(self, known: Known) -> StmtFacts {
        debug_assert!(known.of_statement(), "{known:?} is no statement's fact");
        StmtFacts(self.0.with(known))
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
}
