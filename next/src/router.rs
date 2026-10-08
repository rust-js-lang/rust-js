//! [`next/router`](https://nextjs.org/docs/pages/api-reference/functions/use-router):
//! the Pages Router's route a component is on, and going to another, as
//! react.dev's pages use it. The App Router's is `next/navigation`'s.

use core::marker::PhantomData;

use js::JsObject;

unsafe extern "Rust" {
    /// [`Router`](https://nextjs.org/docs/pages/api-reference/functions/use-router#router-object),
    /// next/router's default export: the router, outside a component.
    #[link_name = "next/router#default"]
    pub safe static Router: &'static NextRouter;
}

#[cfg_attr(rust_js, rust_js::link_name = "next/router#useRouter")]
pub fn use_router() -> &'static NextRouter {
    unreachable!()
}

/// [`NextRouter`](https://nextjs.org/docs/pages/api-reference/functions/use-router#router-object).
pub struct NextRouter(PhantomData<JsObject>);

impl NextRouter {
    /// The path as the browser shows it, its query and hash too.
    #[cfg_attr(rust_js, rust_js::link_name = "get asPath")]
    pub fn as_path(&self) -> &'static str {
        unreachable!()
    }

    /// The page's route, `/blog/[slug]`.
    #[cfg_attr(rust_js, rust_js::link_name = "get pathname")]
    pub fn pathname(&self) -> &'static str {
        unreachable!()
    }

    /// Where the app is served from, under its host.
    #[cfg_attr(rust_js, rust_js::link_name = "get basePath")]
    pub fn base_path(&self) -> &'static str {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    pub fn push(&self, href: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    pub fn replace(&self, href: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "back")]
    pub fn back(&self) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "reload")]
    pub fn reload(&self) {
        unreachable!()
    }

    /// What it emits as it goes to another route, `router.events`.
    #[cfg_attr(rust_js, rust_js::link_name = "get events")]
    pub fn events(&self) -> &'static MittEmitter {
        unreachable!()
    }
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
