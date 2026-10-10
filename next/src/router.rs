//! [`next/router`](https://nextjs.org/docs/pages/api-reference/functions/use-router):
//! the Pages Router's route a component is on, and going to another, as
//! react.dev's pages use it. The App Router's is `next/navigation`'s.

use core::marker::PhantomData;
use core::ops::Deref;

use js::{Dict, JsObject, Promise};
use react::{ComponentType, ComponentValue};

use crate::QueryValue;
use crate::link::LinkLocale;

unsafe extern "Rust" {
    /// [`Router`](https://nextjs.org/docs/pages/api-reference/functions/use-router#router-object),
    /// next/router's default export: the router, outside a component.
    #[link_name = "next/router#default"]
    pub safe static Router: &'static SingletonRouter;
}

#[cfg_attr(rust_js, rust_js::link_name = "next/router#useRouter")]
pub fn use_router() -> &'static NextRouter {
    unreachable!()
}

/// The props of a component [`with_router`] wraps: those it's given, and
/// the router.
pub struct WithRouterProps<P> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: P,
    pub router: &'static NextRouter,
}

/// [`withRouter`](https://nextjs.org/docs/pages/api-reference/functions/use-router#withrouter),
/// in a `thread_local!`: a component given `P`, which renders `component`
/// with the router too, as react.dev's `Seo` is made.
#[cfg_attr(rust_js, rust_js::link_name = "next/router#withRouter")]
#[allow(unused_variables)]
pub fn with_router<P, M>(component: impl ComponentType<WithRouterProps<P>, M>) -> ComponentValue<P> {
    unreachable!()
}

/// [`NextRouter`](https://nextjs.org/docs/pages/api-reference/functions/use-router#router-object).
pub struct NextRouter(PhantomData<JsObject>);

impl NextRouter {
    /// The page's route, `/blog/[slug]`.
    #[cfg_attr(rust_js, rust_js::link_name = "get route")]
    pub fn route(&self) -> &'static str {
        unreachable!()
    }

    /// The page's path under its base path, `/blog/[slug]`.
    #[cfg_attr(rust_js, rust_js::link_name = "get pathname")]
    pub fn pathname(&self) -> &'static str {
        unreachable!()
    }

    /// The query, and the dynamic route's parameters.
    #[cfg_attr(rust_js, rust_js::link_name = "get query")]
    pub fn query(&self) -> &'static Dict<QueryValue> {
        unreachable!()
    }

    /// The path as the browser shows it, its query and hash too.
    #[cfg_attr(rust_js, rust_js::link_name = "get asPath")]
    pub fn as_path(&self) -> &'static str {
        unreachable!()
    }

    /// Where the app is served from, under its host.
    #[cfg_attr(rust_js, rust_js::link_name = "get basePath")]
    pub fn base_path(&self) -> &'static str {
        unreachable!()
    }

    /// The locale it's in, of i18n routing.
    #[cfg_attr(rust_js, rust_js::link_name = "get locale")]
    pub fn locale(&self) -> Option<&'static str> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get locales")]
    pub fn locales(&self) -> Option<Vec<&'static str>> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get defaultLocale")]
    pub fn default_locale(&self) -> Option<&'static str> {
        unreachable!()
    }

    /// Each domain's locales.
    #[cfg_attr(rust_js, rust_js::link_name = "get domainLocales")]
    pub fn domain_locales(&self) -> Option<Vec<DomainLocale>> {
        unreachable!()
    }

    /// Whether its locale is its domain's.
    #[cfg_attr(rust_js, rust_js::link_name = "get isLocaleDomain")]
    pub fn is_locale_domain(&self) -> bool {
        unreachable!()
    }

    /// Whether its fields are filled in, after hydration.
    #[cfg_attr(rust_js, rust_js::link_name = "get isReady")]
    pub fn is_ready(&self) -> bool {
        unreachable!()
    }

    /// Whether it's a fallback page's, its props not here yet.
    #[cfg_attr(rust_js, rust_js::link_name = "get isFallback")]
    pub fn is_fallback(&self) -> bool {
        unreachable!()
    }

    /// Whether it's in preview mode.
    #[cfg_attr(rust_js, rust_js::link_name = "get isPreview")]
    pub fn is_preview(&self) -> bool {
        unreachable!()
    }

    /// Go to `url`, adding to the history: whether it went.
    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    pub fn push(&self, url: &str) -> Promise<bool> {
        unreachable!()
    }

    /// [`push`](Self::push), the browser showing `r#as`, of `options`.
    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    pub fn push_with_options(&self, url: &str, r#as: Option<&str>, options: TransitionOptions<'_>) -> Promise<bool> {
        unreachable!()
    }

    /// Go to `url`, replacing the history's entry: whether it went.
    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    pub fn replace(&self, url: &str) -> Promise<bool> {
        unreachable!()
    }

    /// [`replace`](Self::replace), the browser showing `r#as`, of `options`.
    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    pub fn replace_with_options(&self, url: &str, r#as: Option<&str>, options: TransitionOptions<'_>) -> Promise<bool> {
        unreachable!()
    }

    /// Load the page again.
    #[cfg_attr(rust_js, rust_js::link_name = "reload")]
    pub fn reload(&self) {
        unreachable!()
    }

    /// Go back in the history.
    #[cfg_attr(rust_js, rust_js::link_name = "back")]
    pub fn back(&self) {
        unreachable!()
    }

    /// Go forward in the history.
    #[cfg_attr(rust_js, rust_js::link_name = "forward")]
    pub fn forward(&self) {
        unreachable!()
    }

    /// Load `url`'s page ahead, for a quicker `push`.
    #[cfg_attr(rust_js, rust_js::link_name = "prefetch")]
    pub fn prefetch(&self, url: &str) -> Promise<()> {
        unreachable!()
    }

    /// [`prefetch`](Self::prefetch), the browser showing `as_path`, of `options`.
    #[cfg_attr(rust_js, rust_js::link_name = "prefetch")]
    pub fn prefetch_with_options(&self, url: &str, as_path: Option<&str>, options: PrefetchOptions<'_>) -> Promise<()> {
        unreachable!()
    }

    /// Before going back or forward, `cb` of where: `false` stays.
    #[cfg_attr(rust_js, rust_js::link_name = "beforePopState")]
    pub fn before_pop_state(&self, cb: impl Fn(&NextHistoryState) -> bool + 'static) {
        unreachable!()
    }

    /// What it emits as it goes to another route, `router.events`.
    #[cfg_attr(rust_js, rust_js::link_name = "get events")]
    pub fn events(&self) -> &'static MittEmitter {
        unreachable!()
    }
}

/// [`SingletonRouter`](https://nextjs.org/docs/pages/api-reference/functions/use-router#router-object),
/// next/router's default export: a [`NextRouter`], and the router it's of.
pub struct SingletonRouter(PhantomData<JsObject>);

impl Deref for SingletonRouter {
    type Target = NextRouter;

    fn deref(&self) -> &NextRouter {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const NextRouter) }
    }
}

impl SingletonRouter {
    /// The router, once there's one, in the browser.
    #[cfg_attr(rust_js, rust_js::link_name = "get router")]
    pub fn router(&self) -> Option<&'static NextRouter> {
        unreachable!()
    }

    /// What runs once there's a router.
    #[cfg_attr(rust_js, rust_js::link_name = "get readyCallbacks")]
    pub fn ready_callbacks(&self) -> Vec<&'static dyn Fn()> {
        unreachable!()
    }

    /// Call `cb` once there's a router.
    #[cfg_attr(rust_js, rust_js::link_name = "ready")]
    pub fn ready(&self, cb: impl Fn() + 'static) {
        unreachable!()
    }
}

/// How a [`NextRouter::push_with_options`] goes.
#[derive(Default)]
pub struct TransitionOptions<'a> {
    /// Without the page's data fetched again.
    pub shallow: Option<bool>,
    /// The locale it goes to, or `LinkLocale::Bool(false)`, none.
    pub locale: Option<LinkLocale<'a>>,
    /// Scroll to the top of the new page: `true`.
    pub scroll: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "unstable_skipClientCache")]
    pub unstable_skip_client_cache: Option<bool>,
}

/// How a [`NextRouter::prefetch_with_options`] loads.
#[derive(Default)]
pub struct PrefetchOptions<'a> {
    pub priority: Option<bool>,
    pub locale: Option<LinkLocale<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "unstable_skipClientCache")]
    pub unstable_skip_client_cache: Option<bool>,
}

/// Where going back or forward goes, what [`NextRouter::before_pop_state`]'s
/// callback is given.
pub struct NextHistoryState(PhantomData<JsObject>);

impl NextHistoryState {
    #[cfg_attr(rust_js, rust_js::link_name = "get url")]
    pub fn url(&self) -> &'static str {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get as")]
    pub fn r#as(&self) -> &'static str {
        unreachable!()
    }
}

/// A domain's locales, of i18n routing.
pub struct DomainLocale {
    #[cfg_attr(rust_js, rust_js::name = "defaultLocale")]
    pub default_locale: String,
    pub domain: String,
    pub http: Option<bool>,
    pub locales: Option<Vec<String>>,
}

/// An instance of next/router's `Router` class, as [`SingletonRouter::router`]
/// gives one: a [`NextRouter`], the members Next.js makes public of it.
pub type Router = NextRouter;

unsafe extern "Rust" {
    /// The `Router` class's emitter, `Router.events`: each router's.
    #[link_name = "next/router#Router.events"]
    pub safe static RouterEvents: &'static MittEmitter;
}

/// [`router.events`](https://nextjs.org/docs/pages/api-reference/functions/use-router#routerevents),
/// Next.js's emitter, `mitt`'s.
pub struct MittEmitter(PhantomData<JsObject>);

/// An event of [`MittEmitter`]'s, as @types' `RouterEvent`: each whose
/// handler is given the URL it goes to first. `routeChangeError`'s is
/// given its error first, so isn't one.
#[derive(Clone, Copy)]
pub enum RouterEvent {
    #[cfg_attr(rust_js, rust_js::name = "routeChangeStart")]
    RouteChangeStart,
    #[cfg_attr(rust_js, rust_js::name = "beforeHistoryChange")]
    BeforeHistoryChange,
    #[cfg_attr(rust_js, rust_js::name = "routeChangeComplete")]
    RouteChangeComplete,
    #[cfg_attr(rust_js, rust_js::name = "hashChangeStart")]
    HashChangeStart,
    #[cfg_attr(rust_js, rust_js::name = "hashChangeComplete")]
    HashChangeComplete,
}

impl MittEmitter {
    /// `events.on(type, handler)`: `handler` of each `type` of event, given
    /// the URL, until [`off`](Self::off) is given the same function.
    #[cfg_attr(rust_js, rust_js::link_name = "on")]
    pub fn on(&self, r#type: RouterEvent, handler: &'static dyn Fn(&str)) {
        unreachable!()
    }

    /// `events.off(type, handler)`: `handler` of no more of them.
    #[cfg_attr(rust_js, rust_js::link_name = "off")]
    pub fn off(&self, r#type: RouterEvent, handler: &'static dyn Fn(&str)) {
        unreachable!()
    }
}
