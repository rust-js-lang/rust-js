//! [`next/headers`](https://nextjs.org/docs/app/api-reference/functions/headers):
//! the request a Server Component, a Server Action or a Route Handler is
//! rendered for, its headers and cookies, and whether it's a draft.

use core::marker::PhantomData;

use js::{Date, JsObject, Promise};
use react::webapi::Headers;

/// The request's headers, read only, as `await headers()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/headers#headers")]
pub fn headers() -> Promise<&'static Headers> {
    unreachable!()
}

/// The request's cookies, as `await cookies()`: read in a Server
/// Component, and set or deleted in a Server Action or a Route Handler.
#[cfg_attr(rust_js, rust_js::link_name = "next/headers#cookies")]
pub fn cookies() -> Promise<&'static ReadonlyRequestCookies> {
    unreachable!()
}

/// Whether the request is a draft, and turning it on or off, as
/// `await draftMode()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/headers#draftMode")]
pub fn draft_mode() -> Promise<&'static DraftMode> {
    unreachable!()
}

/// What [`draft_mode`] gives.
pub struct DraftMode(PhantomData<JsObject>);

impl DraftMode {
    /// Whether the request is a draft, `draftMode().isEnabled`.
    #[cfg_attr(rust_js, rust_js::link_name = "get isEnabled")]
    pub fn is_enabled(&self) -> bool {
        unreachable!()
    }

    /// Make the requests after this one drafts, by a cookie.
    #[cfg_attr(rust_js, rust_js::link_name = "enable")]
    pub fn enable(&self) {
        unreachable!()
    }

    /// Make the requests after this one drafts no more.
    #[cfg_attr(rust_js, rust_js::link_name = "disable")]
    pub fn disable(&self) {
        unreachable!()
    }
}

/// What [`cookies`] gives, as Next.js types it: a request's cookies, which
/// it reads, and the response's, which it sets and deletes.
pub struct ReadonlyRequestCookies(PhantomData<JsObject>);

impl ReadonlyRequestCookies {
    /// How many cookies the request has.
    #[cfg_attr(rust_js, rust_js::link_name = "get size")]
    pub fn size(&self) -> f64 {
        unreachable!()
    }

    /// The cookie named `name`, `None` where there's none.
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    pub fn get(&self, name: &str) -> Option<RequestCookie<'static>> {
        unreachable!()
    }

    /// Each cookie.
    #[cfg_attr(rust_js, rust_js::link_name = "getAll")]
    pub fn get_all(&self) -> Vec<RequestCookie<'static>> {
        unreachable!()
    }

    /// Each cookie named `name`.
    #[cfg_attr(rust_js, rust_js::link_name = "getAll")]
    pub fn get_all_with_name(&self, name: &str) -> Vec<RequestCookie<'static>> {
        unreachable!()
    }

    /// Whether there's a cookie named `name`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    pub fn has(&self, name: &str) -> bool {
        unreachable!()
    }

    /// Set the response's cookie `key` to `value`.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set(&self, key: &str, value: &str) -> &Self {
        unreachable!()
    }

    /// Set the response's cookie `key` to `value`, of `cookie`'s options.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set_with_options(&self, key: &str, value: &str, cookie: CookieOptions<'_>) -> &Self {
        unreachable!()
    }

    /// Set the response's cookie, `options` its name, value and options.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set_cookie(&self, options: ResponseCookie<'_>) -> &Self {
        unreachable!()
    }

    /// Delete the cookie `key`, by the response's expiring it.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    pub fn delete(&self, key: &str) -> &Self {
        unreachable!()
    }

    /// Delete the cookie `options` names, of its domain and path.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    pub fn delete_cookie(&self, options: DeletedCookie<'_>) -> &Self {
        unreachable!()
    }

    /// The cookies as a `Cookie` header has them, `a=1; b=2`.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }
}

/// A request's cookie, as `@edge-runtime/cookies` types it.
pub struct RequestCookie<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

/// A response's cookie, as `@edge-runtime/cookies` types it: its name and
/// value, and its options.
pub struct ResponseCookie<'a> {
    pub name: &'a str,
    pub value: &'a str,
    /// When it expires: a time in milliseconds, or a date.
    pub expires: Option<Expires<'a>>,
    pub domain: Option<&'a str>,
    pub path: Option<&'a str>,
    pub secure: Option<bool>,
    /// `"lax"`, `"strict"` or `"none"`, or `true`, which is `"strict"`.
    #[cfg_attr(rust_js, rust_js::name = "sameSite")]
    pub same_site: Option<SameSite<'a>>,
    pub partitioned: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "httpOnly")]
    pub http_only: Option<bool>,
    /// How long it lasts, in seconds.
    #[cfg_attr(rust_js, rust_js::name = "maxAge")]
    pub max_age: Option<f64>,
    /// `"low"`, `"medium"` or `"high"`.
    pub priority: Option<&'a str>,
}

/// A [`ResponseCookie`]'s options, each `None` but what's given, as
/// `Partial<ResponseCookie>` is a set cookie's third argument.
#[derive(Default)]
pub struct CookieOptions<'a> {
    pub expires: Option<Expires<'a>>,
    pub domain: Option<&'a str>,
    pub path: Option<&'a str>,
    pub secure: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "sameSite")]
    pub same_site: Option<SameSite<'a>>,
    pub partitioned: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "httpOnly")]
    pub http_only: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "maxAge")]
    pub max_age: Option<f64>,
    pub priority: Option<&'a str>,
}

/// A deleted cookie, as `Omit<ResponseCookie, 'value' | 'expires'>`: its
/// name, and the domain and path it's of.
pub struct DeletedCookie<'a> {
    pub name: &'a str,
    pub domain: Option<&'a str>,
    pub path: Option<&'a str>,
    pub secure: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "sameSite")]
    pub same_site: Option<SameSite<'a>>,
    pub partitioned: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "httpOnly")]
    pub http_only: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "maxAge")]
    pub max_age: Option<f64>,
    pub priority: Option<&'a str>,
}

/// When a cookie expires, `number | Date`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Expires<'a> {
    /// A time, in milliseconds since 1970.
    Number(f64),
    Date(&'a Date),
}

/// A cookie's `sameSite`, `true | false | "lax" | "strict" | "none"`: each
/// the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum SameSite<'a> {
    Bool(bool),
    Str(&'a str),
}
