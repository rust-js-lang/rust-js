//! [React's `Children`](https://react.dev/reference/react/Children): a
//! component's children, looked inside, as react.dev's MDX components look
//! for a `<pre>` or an `<h4>` among theirs.

use js::JsObject;

use super::{ReactNode, ReactElement, sealed};

/// A child as [`to_array`] gives one: text, a number, an element, or another
/// node, a portal say, told apart as an untagged enum is (ADR 0214).
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy)]
pub enum Child<'a> {
    Text(&'a str),
    Number(f64),
    Element(&'a ReactElement),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(&'a JsObject),
}

impl ReactNode for Child<'_> {}
impl sealed::Sealed for Child<'_> {}

/// [`Children.toArray(children)`](https://react.dev/reference/react/Children#children-toarray):
/// each child, a list's flattened, `null`, `undefined` and booleans left
/// out, each keyed.
#[cfg_attr(rust_js, rust_js::link_name = "react#Children.toArray")]
#[allow(unused_variables)]
pub fn to_array<C: ReactNode>(children: &C) -> Vec<Child<'_>> {
    unreachable!()
}

/// [`Children.forEach(children, f)`](https://react.dev/reference/react/Children#children-foreach):
/// `f` of each child, as [`to_array`] has them. Each is a JS value, kept as
/// long as it's held, as react.dev's Challenges keeps them.
#[cfg_attr(rust_js, rust_js::link_name = "react#Children.forEach")]
#[allow(unused_variables)]
pub fn for_each<C: ReactNode>(children: &C, f: impl FnMut(Child<'static>)) {
    unreachable!()
}
