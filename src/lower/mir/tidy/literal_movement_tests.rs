//! Stability of reads before another expression in a call: fresh
//! literals, and plain values' fields.
use super::read_past_calls;
use rust_js_program::collections::IndexSet;
use rust_js_program::js::{self, Expr, Prop, StmtKind};

#[test]
fn fresh_literals_require_stable_contents() {
    let set: IndexSet<rust_js_program::js::Name> = IndexSet::from_iter(["changing".into()]);
    let none = IndexSet::default();
    let plain = super::Plain::new(&none, None, IndexSet::default());
    let allowed = |value: Expr| read_past_calls(&value, &set, &plain);
    let field = |value: Expr| Expr::object(vec![Prop::Field("entry".to_owned(), value)]);
    assert!(allowed(field(Expr::array(vec![Expr::var("stable")]))));
    assert!(allowed(Expr::array(vec![field(Expr::var("stable"))])));
    for value in [
        field(Expr::var("changing")),
        field(Expr::member(Expr::var("external"), "entry")),
        field(Expr::call(Expr::var("external"), Vec::new())),
        Expr::object(vec![Prop::Spread(Expr::var("external"))]),
        Expr::object(vec![Prop::Keyed(Expr::var("key"), Expr::var("stable"))]),
        Expr::array(vec![Expr::spread(Expr::var("external"))]),
        Expr::array(vec![Expr::var("changing")]),
    ] {
        assert!(!allowed(value));
    }
}

/// A plain value's field, `q.title`, or an import's, `tags.link`, reads the
/// same past a call; not an import whose fields change in place (#1028),
/// nor a parameter of a function called at once that names a plain value's
/// variable apart, `((q) => q.title)(other)`: the three sets tidy asks of
/// as one (#2315).
#[test]
fn plain_reads_are_own_and_imports_less_changing_and_shadowed() {
    let none: IndexSet<js::Name> = IndexSet::default();
    let q: js::Name = "q".into();
    let (tags, cells) = (js::Name::module("tags"), js::Name::module("cells"));
    let own = IndexSet::from_iter([js::Path::var(&q)]);
    let imports = IndexSet::from_iter([js::Path::var(&tags), js::Path::var(&cells)]);
    let changing = IndexSet::from_iter([js::Path::var(&cells)]);
    let plain = super::Plain::new(&own, Some(&imports), changing);
    let past = |value: Expr| read_past_calls(&value, &none, &plain);
    let title = |of: &js::Name| Expr::member(Expr::var(of), "title");
    assert!(past(title(&q)));
    assert!(past(title(&tags)));
    assert!(!past(title(&cells)));
    let returned = vec![StmtKind::Return(Some(title(&q))).at(js::Span::NONE)];
    let called = Expr::call(
        Expr::arrow(vec![js::Pattern::Name(q.clone())], returned),
        vec![Expr::var("other")],
    );
    assert!(!past(called));
}
