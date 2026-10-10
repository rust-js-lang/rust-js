//! [`next/app`](https://nextjs.org/docs/pages/building-your-application/routing/custom-app):
//! the Pages Router's app, `pages/_app`, which renders each page in it.

use core::marker::PhantomData;

use js::{Dict, JsObject, Promise, Unknown};
use react::{ComponentValue, ElementType};

use crate::{NextComponentType, NextPageContext};
use crate::router::NextRouter;

/// [`AppProps`](https://nextjs.org/docs/pages/building-your-application/routing/custom-app):
/// what the app is given, the page's component and the props its data
/// fetching gave, `<Component {...pageProps} />`.
pub struct AppProps<P = &'static Unknown> {
    #[cfg_attr(rust_js, rust_js::name = "Component")]
    pub component: ElementType,
    #[cfg_attr(rust_js, rust_js::name = "pageProps")]
    pub page_props: P,
    pub router: &'static NextRouter,
}

/// What Next.js gives the app's `getInitialProps`, as `AppContext` types it.
pub struct AppContext(PhantomData<JsObject>);

impl AppContext {
    /// The page's component.
    #[cfg_attr(rust_js, rust_js::link_name = "get Component")]
    pub fn component(&self) -> ElementType {
        unreachable!()
    }

    /// The app's tree, a component a page may render again of.
    #[cfg_attr(rust_js, rust_js::link_name = "get AppTree")]
    pub fn app_tree(&self) -> &'static ComponentValue<Unknown> {
        unreachable!()
    }

    /// The page's context, its `getInitialProps`'.
    #[cfg_attr(rust_js, rust_js::link_name = "get ctx")]
    pub fn ctx(&self) -> &'static NextPageContext {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get router")]
    pub fn router(&self) -> &'static NextRouter {
        unreachable!()
    }
}

/// What the app's `getInitialProps` gives, as `AppInitialProps` types it:
/// the page's props.
pub struct AppInitialProps<P = &'static Unknown> {
    #[cfg_attr(rust_js, rust_js::name = "pageProps")]
    pub page_props: P,
}

/// An app, as `AppType` types it: a component of its [`AppProps`].
pub trait AppType<P, M>: NextComponentType<AppProps<P>, M> {}

impl<T: NextComponentType<AppProps<P>, M>, P, M> AppType<P, M> for T {}

/// [`App`](https://nextjs.org/docs/pages/building-your-application/routing/custom-app),
/// next/app's default export: Next.js's own app, its statics.
pub mod app {
    use super::*;

    /// `App.getInitialProps(context)`: what Next.js's own app gives, its
    /// page's `getInitialProps`', for a custom one's to add to.
    #[cfg_attr(rust_js, rust_js::link_name = "next/app#default.getInitialProps")]
    pub fn get_initial_props(context: &AppContext) -> Promise<AppInitialProps> {
        unreachable!()
    }

    /// `App.origGetInitialProps(context)`: Next.js's own app's, as it was
    /// before a custom app's replaced it.
    #[cfg_attr(rust_js, rust_js::link_name = "next/app#default.origGetInitialProps")]
    pub fn orig_get_initial_props(context: &AppContext) -> Promise<AppInitialProps> {
        unreachable!()
    }
}

/// A Web Vital Next.js measured, or one of its own, as `NextWebVitalsMetric`
/// types it: what a custom app's `reportWebVitals` is given.
pub struct NextWebVitalsMetric(PhantomData<JsObject>);

impl NextWebVitalsMetric {
    #[cfg_attr(rust_js, rust_js::link_name = "get id")]
    pub fn id(&self) -> &'static str {
        unreachable!()
    }

    /// When it started, in milliseconds since the page did.
    #[cfg_attr(rust_js, rust_js::link_name = "get startTime")]
    pub fn start_time(&self) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get value")]
    pub fn value(&self) -> f64 {
        unreachable!()
    }

    /// What it's of, by its names.
    #[cfg_attr(rust_js, rust_js::link_name = "get attribution")]
    pub fn attribution(&self) -> Option<&'static Dict<Unknown>> {
        unreachable!()
    }

    /// `"web-vital"`, or `"custom"`, Next.js's own.
    #[cfg_attr(rust_js, rust_js::link_name = "get label")]
    pub fn label(&self) -> &'static str {
        unreachable!()
    }

    /// A Web Vital's, `"LCP"`, or Next.js's, `"Next.js-hydration"`.
    #[cfg_attr(rust_js, rust_js::link_name = "get name")]
    pub fn name(&self) -> &'static str {
        unreachable!()
    }
}
