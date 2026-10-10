//! [`next/link`](https://nextjs.org/docs/app/api-reference/components/link):
//! an `<a>` that goes to another route without loading the page again.

use core::marker::PhantomData;

use js::JsObject;
use react::attributes::AnchorHTMLAttributes;
use react::webapi::HTMLAnchorElement;
use react::{JSX, ReactNode, RefObject};

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
    /// Load the route ahead, as the link is seen, in production: all of it,
    /// `Prefetch::Bool(true)`, none, or `Prefetch::Auto`, its default.
    pub prefetch: Option<Prefetch>,
    /// The path the browser shows, where it isn't `href`'s: the Pages
    /// Router's, deprecated.
    #[cfg_attr(rust_js, rust_js::name = "as")]
    pub r#as: Option<&'a str>,
    /// Go there without its `getStaticProps` or `getServerSideProps` run
    /// again: the Pages Router's.
    pub shallow: Option<bool>,
    /// The locale it goes to, or `LinkLocale::Bool(false)`, none: the Pages
    /// Router's i18n.
    pub locale: Option<LinkLocale<'a>>,
    /// Its child is the `<a>`, as before Next.js 13: deprecated.
    #[cfg_attr(rust_js, rust_js::name = "legacyBehavior")]
    pub legacy_behavior: Option<bool>,
    /// Called as it goes, on the client: `event.prevent_default()` stays.
    #[cfg_attr(rust_js, rust_js::name = "onNavigate")]
    pub on_navigate: Option<Box<dyn Fn(&NavigateEvent)>>,
    /// The transition types React's `<ViewTransition>`s animate it by.
    #[cfg_attr(rust_js, rust_js::name = "transitionTypes")]
    pub transition_types: Option<&'a [&'a str]>,
    /// Load the route ahead as it's hovered, all of it.
    #[cfg_attr(rust_js, rust_js::name = "unstable_dynamicOnHover")]
    pub unstable_dynamic_on_hover: Option<bool>,
    /// Send `href` to its child, as a child that isn't an `<a>` needs: `false`.
    #[cfg_attr(rust_js, rust_js::name = "passHref")]
    pub pass_href: Option<bool>,
    /// Its `<a>`, as React's `RefAttributes<HTMLAnchorElement>` holds it.
    pub r#ref: Option<RefObject<Option<&'static HTMLAnchorElement>>>,
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
    /// What it shows, last, as Next.js types it and JSX makes it: after
    /// the props, which a call may make.
    pub children: C,
}

/// Whether the [`Link`] it's rendered in is going to its route, as
/// `useLinkStatus()`: a component's in a `Link`'s children, a client
/// component's.
#[cfg_attr(rust_js, rust_js::link_name = "next/link#useLinkStatus")]
pub fn use_link_status() -> LinkStatus {
    unreachable!()
}

/// What [`use_link_status`] gives.
pub struct LinkStatus {
    /// Going to the link's route, not there yet.
    pub pending: bool,
}

/// A [`LinkProps`]'s `prefetch`, `boolean | "auto"`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Prefetch {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "auto")]
    Auto,
}

/// A [`LinkProps`]'s `locale`, `string | false`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum LinkLocale<'a> {
    Str(&'a str),
    Bool(bool),
}

/// What a [`LinkProps`]'s `on_navigate` is given.
pub struct NavigateEvent(PhantomData<JsObject>);

impl NavigateEvent {
    /// Stay where it is.
    #[cfg_attr(rust_js, rust_js::link_name = "preventDefault")]
    pub fn prevent_default(&self) {
        unreachable!()
    }
}
