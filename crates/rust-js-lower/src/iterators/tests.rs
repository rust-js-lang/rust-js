//! `iterators`' own unit tests.

use super::*;

/// What `iterated` gives of a value it doesn't unwrap is that value,
/// every fact its makers said of it kept: `entries(it.map(f))`'s
/// `$mapEnds(..)` kept its `Form` facts, read past it (#2602).
#[test]
fn iterated_keeps_what_its_makers_said() {
    let made = Expr::call_helper("$mapEnds", vec![Expr::int(1), Expr::int(2)]);
    let made = Expr {
        facts: made
            .facts
            .with(js::Known::Iterator)
            .with(js::Known::InPlace)
            .with(js::Known::IterKnowsBack),
        ..made
    };
    let facts = made.facts;
    assert!(iterated(made).facts == facts);
}
