//! [`next/link`](https://nextjs.org/docs/app/api-reference/components/link):
//! an `<a>` that goes to another route without loading the page again.

use react::{Element, Node};

/// `<Link href="/about" {..Default::default()}>{"About"}</Link>`.
#[cfg_attr(rust_js, rust_js::link_name = "next/link#default")]
pub fn Link<C: Node>(props: LinkProps<C>) -> Element {
    unreachable!()
}

/// What a [`Link`] is given: `href`, its children, text or elements, and
/// each of the rest `None`, which Next.js takes as its default.
#[derive(Default)]
pub struct LinkProps<C> {
    /// The route it goes to, `/about`, or another site's URL.
    pub href: &'static str,
    /// What it shows.
    pub children: C,
    /// Replace the history's entry, not add one.
    pub replace: Option<bool>,
    /// Scroll to the top of the new page, or keep where it is: `true`.
    pub scroll: Option<bool>,
    /// Load the route ahead, as the link is seen: in production only.
    pub prefetch: Option<bool>,
    /// The `<a>`'s classes.
    #[cfg_attr(rust_js, rust_js::name = "className")]
    pub class_name: Option<&'static str>,
    /// The `<a>`'s `target`, `"_blank"`.
    pub target: Option<&'static str>,
    /// The `<a>`'s `rel`, `"noopener noreferrer"`.
    pub rel: Option<&'static str>,
    /// The `<a>`'s `id`.
    pub id: Option<&'static str>,
}
