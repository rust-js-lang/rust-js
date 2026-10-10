//! [`next/form`](https://nextjs.org/docs/app/api-reference/components/form):
//! a `<form>` that goes to its route without loading the page again.

use react::attributes::HTMLAttributes;
use react::webapi::HTMLFormElement;
use react::{FormAction, JSX, ReactNode, RefObject};

/// `<Form action="/search"><input name="query" /></Form>`.
#[cfg_attr(rust_js, rust_js::link_name = "next/form#default")]
pub fn Form<C: ReactNode, A: FormAction<M>, M>(props: FormProps<'_, C, A>) -> JSX::Element {
    unreachable!()
}

/// What a [`Form`] is given, as Next.js types it: its `action`, and a
/// `<form>`'s attributes but `method`, `encType` and `target`, which
/// Next.js sets.
pub struct FormProps<'a, C, A> {
    /// Where it goes: a route, whose query its fields are, or a function,
    /// given its data.
    pub action: A,
    /// Load the route ahead, as it's seen, unless it's `Some(false)`.
    pub prefetch: Option<bool>,
    /// Replace the history's entry, not add one.
    pub replace: Option<bool>,
    /// Scroll to the top of the new page, or keep where it is: `true`.
    pub scroll: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "acceptCharset")]
    pub accept_charset: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "autoComplete")]
    pub auto_complete: Option<&'a str>,
    pub name: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "noValidate")]
    pub no_validate: Option<bool>,
    /// Its `<form>`, as React's `RefAttributes<HTMLFormElement>` holds it.
    pub r#ref: Option<RefObject<Option<&'static HTMLFormElement>>>,
    /// The `<form>`'s other attributes, as React's `HTMLAttributes`.
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub html: HTMLAttributes<'a, HTMLFormElement>,
    /// Its fields.
    pub children: C,
}
