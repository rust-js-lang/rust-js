//! The one JS form of a Rust iterator a JS iterator steps, `$iterator(value,
//! next, ..)`: its arguments, given what the iterator's impl writes of its
//! own (`OwnMethods`), each left out at the end where it's the default.

use super::*;

/// `$iterator(value, next)`: a JS iterator of `value`, a Rust iterator,
/// stepped by `next`, which gives its `Option`; of one whose `Some` may be
/// boxed (ADR 0051), `boxed`, unboxed; given `back`, its `next_back`, one
/// that knows its back, which `$nextBack` steps (ADR 0429). The one JS form
/// of it.
pub fn stepped(value: Expr, next: Expr, boxed: bool, back: Option<Expr>) -> Expr {
    stepped_ends(value, next, boxed, back, None, None, OwnMethods::default())
}

/// A `$iterator` call's arguments, `given`, without those at the end that
/// are its defaults: an `undefined`, and `boxed`'s `false`.
pub(crate) fn fewest_iterator_args(given: &mut Vec<Expr>) {
    while given.len() > 2 && matches!(given.last().map(|e| &e.kind), Some(js::ExprKind::Undefined)) {
        given.pop();
    }
    if given.len() == 3 && matches!(given[2].kind, js::ExprKind::Bool(false)) {
        given.pop();
    }
}

/// The methods the impl of a crate's iterator writes itself that std
/// routes other calls through, each where it writes it: what `$iterator`
/// is given, and its dictionary (#162, #182); its `rfold` and `nth_back`,
/// which std's adapters stepped from their back route through (#220).
#[derive(Default)]
pub(crate) struct OwnMethods {
    pub(crate) fold: Option<Expr>,
    pub(crate) nth: Option<Expr>,
    pub(crate) last: Option<Expr>,
    pub(crate) rfold: Option<Expr>,
    pub(crate) nth_back: Option<Expr>,
    /// Its own `count`, which a `skip`'s, a `fuse`'s, a `peekable`'s, a
    /// chain's and generic code's `count` call (#505).
    pub(crate) count: Option<Expr>,
}

/// `stepped`'s, given `len` too, its length, one that knows it, an
/// `ExactSizeIterator`'s (ADR 0456), `hint`, its own `size_hint`, an
/// iterator of the crate's whose impl has one: what `size_hint()` of it
/// gives in generic code, as in its own; and `own`, its own `fold` and
/// `nth`, which std's consumers call (#162).
pub(crate) fn stepped_ends(
    value: Expr,
    next: Expr,
    boxed: bool,
    back: Option<Expr>,
    len: Option<Expr>,
    hint: Option<Expr>,
    own: OwnMethods,
) -> Expr {
    // Each given, and each before it, `undefined` where it isn't.
    let OwnMethods {
        fold,
        nth,
        last,
        rfold,
        nth_back,
        count,
    } = own;
    let given = [
        boxed.then(|| Expr::bool(true)),
        back,
        len,
        hint,
        fold,
        nth,
        last,
        rfold,
        nth_back,
        None,
        count,
    ];
    let last = given.iter().rposition(Option::is_some).map_or(0, |at| at + 1);
    let given = (given.into_iter().enumerate().take(last))
        .map(|(at, arg)| {
            arg.unwrap_or_else(|| match at {
                0 => Expr::bool(false),
                _ => Expr::js_undefined(),
            })
        })
        .collect();
    IterValue::of(Form::Own { value, next, given }).emit()
}

/// `$iterator(range, step)`: a range's numbers, its bounds stepped by
/// `step`, its kind's (`range_stepper`).
pub fn stepped_range_by(range: Expr, step: Expr) -> Expr {
    IterValue::of(Form::Range { range, step }).emit()
}
