//! [`next/navigation`](https://nextjs.org/docs/app/api-reference/functions/use-router):
//! the route a client component is on, and going to another. Its hooks are
//! a client component's, `js::directive!("use client");`.

use core::marker::PhantomData;

use js::JsObject;

/// The router, as `useRouter()`: what goes to another route.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useRouter")]
pub fn use_router() -> &'static Router {
    unreachable!()
}

/// The route's path, `/dashboard`, as `usePathname()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#usePathname")]
pub fn use_pathname() -> &'static str {
    unreachable!()
}

/// The URL's query, `?q=rust`, as `useSearchParams()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#useSearchParams")]
pub fn use_search_params() -> &'static SearchParams {
    unreachable!()
}

/// Show the route's `not-found` page, as `notFound()`, which throws.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#notFound")]
pub fn not_found() -> ! {
    unreachable!()
}

/// Go to `url` instead, as `redirect(url)`, which throws.
#[cfg_attr(rust_js, rust_js::link_name = "next/navigation#redirect")]
pub fn redirect(url: &str) -> ! {
    unreachable!()
}

/// What [`use_router`] gives.
pub struct Router(PhantomData<JsObject>);

impl Router {
    /// Go to `href`, adding to the history.
    #[cfg_attr(rust_js, rust_js::link_name = "push")]
    pub fn push(&self, href: &str) {
        unreachable!()
    }

    /// Go to `href`, replacing the history's entry.
    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    pub fn replace(&self, href: &str) {
        unreachable!()
    }

    /// Render the route again, its Server Components from the server.
    #[cfg_attr(rust_js, rust_js::link_name = "refresh")]
    pub fn refresh(&self) {
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

    /// Load `href` ahead, for a quicker `push`.
    #[cfg_attr(rust_js, rust_js::link_name = "prefetch")]
    pub fn prefetch(&self, href: &str) {
        unreachable!()
    }
}

/// What [`use_search_params`] gives: the query's, read only.
pub struct SearchParams(PhantomData<JsObject>);

impl SearchParams {
    /// The first value of `name`, `None` where there's none.
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    pub fn get(&self, name: &str) -> Option<&'static str> {
        unreachable!()
    }

    /// Whether there's a `name`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    pub fn has(&self, name: &str) -> bool {
        unreachable!()
    }
}
