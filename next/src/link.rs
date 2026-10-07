//! [`next/link`](https://nextjs.org/docs/app/api-reference/components/link):
//! an `<a>` that goes to another route without loading the page again.

use react::attributes::AnchorHTMLAttributes;
use react::{JSX, ReactNode};

/// `<Link href="/about" {..Default::default()}>{"About"}</Link>`.
#[cfg_attr(rust_js, rust_js::link_name = "next/link#default")]
pub fn Link<C: ReactNode>(props: LinkProps<'_, C>) -> JSX::Element {
    unreachable!()
}

/// What a [`Link`] is given: `href`, its children, text or elements, and
/// each of the rest `None`, which Next.js takes as its default. Its text
/// is borrowed, so one a component computes, its classes, is given too.
#[derive(Default)]
pub struct LinkProps<'a, C> {
    /// The route it goes to, `/about`, or another site's URL.
    pub href: &'a str,
    /// What it shows.
    pub children: C,
    /// The `<a>`'s other props, as Next.js types them, React's
    /// `AnchorHTMLAttributes` (ADR 0208): a component's own, passed on,
    /// `anchor={props}`, is `{...props}` where it's written. Those named
    /// here are these.
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: AnchorHTMLAttributes<'a>,
    /// Replace the history's entry, not add one.
    pub replace: Option<bool>,
    /// Scroll to the top of the new page, or keep where it is: `true`.
    pub scroll: Option<bool>,
    /// Load the route ahead, as the link is seen: in production only.
    pub prefetch: Option<bool>,
    /// The `<a>`'s classes.
    #[cfg_attr(rust_js, rust_js::name = "className")]
    pub class_name: Option<&'a str>,
    /// The `<a>`'s `target`, `"_blank"`.
    pub target: Option<&'a str>,
    /// The `<a>`'s `rel`, `"noopener noreferrer"`.
    pub rel: Option<&'a str>,
    /// The `<a>`'s `id`.
    pub id: Option<&'a str>,
    /// The `<a>`'s `aria-label`, its name for who can't see it.
    #[cfg_attr(rust_js, rust_js::name = "aria-label")]
    pub aria_label: Option<&'a str>,
}
