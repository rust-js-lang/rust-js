//! Stability of fresh literals before another expression in a call.
use super::read_past_calls;
use crate::collections::IndexSet;
use crate::js::{Expr, Prop};

#[test]
fn fresh_literals_require_stable_contents() {
    let set = IndexSet::from_iter(["changing".to_owned()]);
    let plain = IndexSet::default();
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
