//! [`next/navigation`](https://nextjs.org/docs/app/api-reference/functions/use-router):
//! the route a client component is on, and going to another. Its hooks are
//! a client component's, `js::directive!("use client");`.

use core::marker::PhantomData;

use js::{JsObject, Unknown};
use react::{Context, JSX, ReactNode};

/// The router, as `useRouter()`: what goes to another route.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useRouter")]
pub fn use_router() -> &'static AppRouterInstance {
    unreachable!()
}

/// The route's path, `/dashboard`, as `usePathname()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#usePathname")]
pub fn use_pathname() -> &'static str {
    unreachable!()
}

/// The URL's query, `?q=rust`, as `useSearchParams()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useSearchParams")]
pub fn use_search_params() -> &'static ReadonlyURLSearchParams {
    unreachable!()
}

/// The route's dynamic parameters, as `useParams()`: `T`'s fields, each
/// one's name, a `String`, or a `Vec<String>` of a catch-all.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useParams")]
pub fn use_params<T>() -> T {
    unreachable!()
}

/// The segments below the layout it's called in, as
/// `useSelectedLayoutSegments()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useSelectedLayoutSegments")]
pub fn use_selected_layout_segments() -> Vec<String> {
    unreachable!()
}

/// [`use_selected_layout_segments`] of the parallel route `parallel_route_key`.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useSelectedLayoutSegments")]
pub fn use_selected_layout_segments_with_parallel_route_key(parallel_route_key: &str) -> Vec<String> {
    unreachable!()
}

/// The segment just below the layout it's called in, as
/// `useSelectedLayoutSegment()`: `None` where there's none.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useSelectedLayoutSegment")]
pub fn use_selected_layout_segment() -> Option<String> {
    unreachable!()
}

/// [`use_selected_layout_segment`] of the parallel route `parallel_route_key`.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useSelectedLayoutSegment")]
pub fn use_selected_layout_segment_with_parallel_route_key(parallel_route_key: &str) -> Option<String> {
    unreachable!()
}

/// Put what `callback` renders in the document's `<head>`, as it's rendered
/// on the server, as a CSS-in-JS library's styles are.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useServerInsertedHTML")]
pub fn use_server_inserted_html<R: ReactNode>(callback: impl Fn() -> R + 'static) {
    unreachable!()
}

/// What [`ServerInsertedHTMLContext`] holds: given what to render in the
/// `<head>`.
pub type ServerInsertedHTMLHook = Box<dyn Fn(Box<dyn Fn() -> JSX::Element>)>;

unsafe extern "Rust" {
    /// The context [`use_server_inserted_html`] reads.
    #[link_name = "next/navigation#ServerInsertedHTMLContext"]
    pub safe static ServerInsertedHTMLContext: Context<Option<ServerInsertedHTMLHook>>;
}

/// Show the route's `not-found` page, as `notFound()`, which throws.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#notFound")]
pub fn not_found() -> ! {
    unreachable!()
}

/// Show the route's `forbidden` page, a 403, as `forbidden()`, which throws.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#forbidden")]
pub fn forbidden() -> ! {
    unreachable!()
}

/// Show the route's `unauthorized` page, a 401, as `unauthorized()`,
/// which throws.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#unauthorized")]
pub fn unauthorized() -> ! {
    unreachable!()
}

/// Go to `url` instead, as `redirect(url)`, which throws: a 307.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#redirect")]
pub fn redirect(url: &str) -> ! {
    unreachable!()
}

/// [`redirect`], adding to the history or replacing its entry.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#redirect")]
pub fn redirect_with_type(url: &str, r#type: RedirectType) -> ! {
    unreachable!()
}

/// Go to `url` for good, as `permanentRedirect(url)`, which throws: a 308.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#permanentRedirect")]
pub fn permanent_redirect(url: &str) -> ! {
    unreachable!()
}

/// [`permanent_redirect`], adding to the history or replacing its entry.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#permanentRedirect")]
pub fn permanent_redirect_with_type(url: &str, r#type: RedirectType) -> ! {
    unreachable!()
}

/// How a redirect goes, as `RedirectType` names it: each the string it is.
#[derive(Clone, Copy)]
pub enum RedirectType {
    #[cfg_attr(rust_js, rust_js::name = "push")]
    Push,
    #[cfg_attr(rust_js, rust_js::name = "replace")]
    Replace,
}

/// Throw `error` again where it's Next.js's own, as `unstable_rethrow`: a
/// redirect or a not found, caught by a `catch` around a call that throws
/// one.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#unstable_rethrow")]
pub fn unstable_rethrow(error: &Unknown) {
    unreachable!()
}

/// Whether `error` is a Server Action's that the server didn't know.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#unstable_isUnrecognizedActionError")]
pub fn unstable_is_unrecognized_action_error(error: &Unknown) -> bool {
    unreachable!()
}

/// What [`use_router`] gives.
pub struct AppRouterInstance(PhantomData<JsObject>);

impl AppRouterInstance {
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

    /// Render the route again, its Server Components from the server.
    #[cfg_attr(rust_js, rust_js::link_name = "refresh")]
    pub fn refresh(&self) {
        unreachable!()
    }

    /// Go to `href`, adding to the history.
    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    pub fn push(&self, href: &str) {
        unreachable!()
    }

    /// [`push`](Self::push), of `options`.
    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    pub fn push_with_options(&self, href: &str, options: NavigateOptions<'_>) {
        unreachable!()
    }

    /// Go to `href`, replacing the history's entry.
    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    pub fn replace(&self, href: &str) {
        unreachable!()
    }

    /// [`replace`](Self::replace), of `options`.
    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    pub fn replace_with_options(&self, href: &str, options: NavigateOptions<'_>) {
        unreachable!()
    }

    /// Load `href` ahead, for a quicker `push`.
    #[cfg_attr(rust_js, rust_js::link_name = "prefetch")]
    pub fn prefetch(&self, href: &str) {
        unreachable!()
    }

    /// [`prefetch`](Self::prefetch), of `options`.
    #[cfg_attr(rust_js, rust_js::link_name = "prefetch")]
    pub fn prefetch_with_options(&self, href: &str, options: PrefetchOptions) {
        unreachable!()
    }

    /// [`push`](Self::push) as a gesture's, an experiment's: `None` where
    /// Next.js hasn't one.
    #[cfg_attr(rust_js, rust_js::link_name = "get experimental_gesturePush")]
    pub fn experimental_gesture_push(&self) -> Option<&'static dyn Fn(&str, Option<NavigateOptions<'_>>)> {
        unreachable!()
    }

    /// The route segment's id, new where a push or a replace makes it, the
    /// same going back or forward: a `key` that resets a form's state.
    #[cfg_attr(rust_js, rust_js::link_name = "get bfcacheId")]
    pub fn bfcache_id(&self) -> &'static str {
        unreachable!()
    }
}

/// How [`AppRouterInstance::push`] or `replace` goes.
#[derive(Default)]
pub struct NavigateOptions<'a> {
    /// Scroll to the top of the new page, or keep where it is: `true`.
    pub scroll: Option<bool>,
    /// The transition types React's `<ViewTransition>`s animate it by.
    #[cfg_attr(rust_js, rust_js::name = "transitionTypes")]
    pub transition_types: Option<Vec<&'a str>>,
}

/// How [`AppRouterInstance::prefetch`] loads a route.
pub struct PrefetchOptions {
    pub kind: PrefetchKind,
    /// Called when what it loaded is stale.
    #[cfg_attr(rust_js, rust_js::name = "onInvalidate")]
    pub on_invalidate: Option<Box<dyn Fn()>>,
}

/// How much of a route [`AppRouterInstance::prefetch`] loads: each the
/// string it is.
#[derive(Clone, Copy)]
pub enum PrefetchKind {
    /// Its layouts down to the nearest `loading`, or all of a static one.
    #[cfg_attr(rust_js, rust_js::name = "auto")]
    Auto,
    /// All of it.
    #[cfg_attr(rust_js, rust_js::name = "full")]
    Full,
}

/// What [`use_search_params`] gives: a `URLSearchParams` that's read only.
pub struct ReadonlyURLSearchParams(PhantomData<JsObject>);

impl ReadonlyURLSearchParams {
    /// How many parameters there are.
    #[cfg_attr(rust_js, rust_js::link_name = "get size")]
    pub fn size(&self) -> u32 {
        unreachable!()
    }

    /// The first value of `name`, `None` where there's none.
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    pub fn get(&self, name: &str) -> Option<String> {
        unreachable!()
    }

    /// Each value of `name`.
    #[cfg_attr(rust_js, rust_js::link_name = "getAll")]
    pub fn get_all(&self, name: &str) -> Vec<String> {
        unreachable!()
    }

    /// Whether there's a `name`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    pub fn has(&self, name: &str) -> bool {
        unreachable!()
    }

    /// Whether there's a `name` of `value`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    pub fn has_with_value(&self, name: &str, value: &str) -> bool {
        unreachable!()
    }

    /// Call `callback` with each value and its name.
    #[cfg_attr(rust_js, rust_js::link_name = "forEach")]
    pub fn for_each(&self, callback: Box<dyn FnMut(&str, &str)>) {
        unreachable!()
    }

    /// Each name.
    #[cfg_attr(rust_js, rust_js::link_name = "keys")]
    pub fn keys(&self) -> Box<dyn Iterator<Item = String>> {
        unreachable!()
    }

    /// Each value.
    #[cfg_attr(rust_js, rust_js::link_name = "values")]
    pub fn values(&self) -> Box<dyn Iterator<Item = String>> {
        unreachable!()
    }

    /// Each name and its value.
    #[cfg_attr(rust_js, rust_js::link_name = "entries")]
    pub fn entries(&self) -> Box<dyn Iterator<Item = (String, String)>> {
        unreachable!()
    }

    /// The query, `q=rust&page=2`, without its `?`.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }
}
