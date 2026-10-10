//! [Instrumentation](https://nextjs.org/docs/app/guides/instrumentation): what
//! a project's `instrumentation.rs` and `instrumentation-client.rs` are given,
//! a request's error, and a client's route transitions.

use js::{Dict, Unknown};

/// How a client's route transition goes, as `RouterTransitionType` names it.
#[derive(Clone, Copy, PartialEq)]
pub enum RouterTransitionType {
    #[cfg_attr(rust_js, rust_js::name = "push")]
    Push,
    #[cfg_attr(rust_js, rust_js::name = "replace")]
    Replace,
    /// Back or forward in the history.
    #[cfg_attr(rust_js, rust_js::name = "traverse")]
    Traverse,
}

/// How much of the route was loaded ahead, as `RouterTransitionPrefetchIntent`
/// names it.
#[derive(Clone, Copy, PartialEq)]
pub enum RouterTransitionPrefetchIntent {
    #[cfg_attr(rust_js, rust_js::name = "full")]
    Full,
    #[cfg_attr(rust_js, rust_js::name = "auto")]
    Auto,
    #[cfg_attr(rust_js, rust_js::name = "none")]
    None,
}

/// A route transition, as `RouterTransitionEvent` types it.
pub struct RouterTransitionEvent {
    pub id: String,
    /// When, in milliseconds since the page started.
    pub timestamp: f64,
}

/// A route transition starting, what `onRouterTransitionStart` is given, as
/// `RouterTransitionStartEvent` types it.
pub struct RouterTransitionStartEvent {
    pub id: String,
    pub timestamp: f64,
    /// The routes it leaves.
    #[cfg_attr(rust_js, rust_js::name = "fromRoutes")]
    pub from_routes: Vec<String>,
    #[cfg_attr(rust_js, rust_js::name = "prefetchIntent")]
    pub prefetch_intent: Option<RouterTransitionPrefetchIntent>,
}

/// The request whose rendering threw, what `onRequestError` is given second.
pub struct ErrorRequest {
    pub path: String,
    pub method: String,
    pub headers: &'static Dict<Unknown>,
}

/// Where a request's error was, what `onRequestError` is given last.
pub struct RequestErrorContext {
    /// `"Pages Router"` or `"App Router"`.
    #[cfg_attr(rust_js, rust_js::name = "routerKind")]
    pub router_kind: String,
    /// The route's path, `/blog/[slug]`.
    #[cfg_attr(rust_js, rust_js::name = "routePath")]
    pub route_path: String,
    /// `"render"`, `"route"`, `"action"` or `"proxy"`.
    #[cfg_attr(rust_js, rust_js::name = "routeType")]
    pub route_type: String,
    /// What rendered it: `"react-server-components"`,
    /// `"react-server-components-payload"` or `"server-rendering"`.
    #[cfg_attr(rust_js, rust_js::name = "renderSource")]
    pub render_source: Option<String>,
    /// `"on-demand"` or `"stale"`, where it was rendered again.
    #[cfg_attr(rust_js, rust_js::name = "revalidateReason")]
    pub revalidate_reason: Option<String>,
}

/// What `instrumentation.rs` exports, as the `Instrumentation` namespace types
/// it.
#[allow(non_snake_case, non_camel_case_types)]
pub mod Instrumentation {
    use super::*;

    /// `onRequestError`, called with what a request's rendering threw.
    pub type onRequestError = fn(&'static Unknown, &'static ErrorRequest, &'static RequestErrorContext);
}
