//! [`next/server`](https://nextjs.org/docs/app/api-reference/functions/next-request):
//! a Route Handler's and a proxy's request and response, `app/api/route.rs`'s
//! `pub async fn GET(request: &NextRequest) -> &'static Response`.

use core::future::Future;
use core::marker::PhantomData;
use core::ops::Deref;

use js::{JsObject, Promise, Unknown};
use react::webapi::{
    Headers, IntoBodyInit, IntoURLPatternInput, Request, RequestInit, Response, ResponseInit, URL, URLPattern, URLSearchParams,
};

use crate::OneOrMany;
use crate::headers::{CookieOptions, DeletedCookie, RequestCookie, ResponseCookie};

/// What `next/server` gives of `next/og`.
pub use crate::og::ImageResponseOptions;

/// [`NextRequest`](https://nextjs.org/docs/app/api-reference/functions/next-request):
/// a `Request`, its cookies and its URL Next.js's.
pub struct NextRequest(PhantomData<JsObject>);

impl Deref for NextRequest {
    type Target = Request;

    fn deref(&self) -> &Request {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const Request) }
    }
}

impl NextRequest {
    /// Its cookies, as its `Cookie` header has them.
    #[cfg_attr(rust_js, rust_js::link_name = "get cookies")]
    pub fn cookies(&self) -> &'static RequestCookies {
        unreachable!()
    }

    /// Its URL, its path and query Next.js's, the base path and locale apart.
    #[cfg_attr(rust_js, rust_js::link_name = "get nextUrl")]
    pub fn next_url(&self) -> &'static NextURL {
        unreachable!()
    }
}

/// [`NextRequest`]'s constructors.
pub mod next_request {
    use super::*;

    /// `new NextRequest(input)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/server#NextRequest")]
    pub fn new(input: &str) -> &'static NextRequest {
        unreachable!()
    }

    /// `new NextRequest(input, init)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/server#NextRequest")]
    pub fn new_with_init(input: &str, init: RequestInit<'_>) -> &'static NextRequest {
        unreachable!()
    }
}

/// [`NextResponse`](https://nextjs.org/docs/app/api-reference/functions/next-response):
/// a `Response`, its cookies Next.js's, of a body `Body`.
pub struct NextResponse<Body = Unknown>(PhantomData<JsObject>, PhantomData<Body>);

impl<Body> Deref for NextResponse<Body> {
    type Target = Response;

    fn deref(&self) -> &Response {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const Response) }
    }
}

impl<Body> NextResponse<Body> {
    /// The cookies it sets, its `Set-Cookie` headers.
    #[cfg_attr(rust_js, rust_js::link_name = "get cookies")]
    pub fn cookies(&self) -> &'static ResponseCookies {
        unreachable!()
    }
}

/// [`NextResponse`]'s constructors and its statics.
pub mod next_response {
    use super::*;

    /// `new NextResponse()`: an empty one.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/server#NextResponse")]
    pub fn new() -> &'static NextResponse {
        unreachable!()
    }

    /// `new NextResponse(body)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/server#NextResponse")]
    pub fn new_with_body(body: impl IntoBodyInit) -> &'static NextResponse {
        unreachable!()
    }

    /// `new NextResponse(body, init)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/server#NextResponse")]
    pub fn new_with_body_and_init(body: impl IntoBodyInit, init: ResponseInit<'_>) -> &'static NextResponse {
        unreachable!()
    }

    /// `NextResponse.json(body)`: `body` as JSON, its `Content-Type` JSON's.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.json")]
    pub fn json<JsonBody>(body: JsonBody) -> &'static NextResponse<JsonBody> {
        unreachable!()
    }

    /// `NextResponse.json(body, init)`.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.json")]
    pub fn json_with_init<JsonBody>(body: JsonBody, init: ResponseInit<'_>) -> &'static NextResponse<JsonBody> {
        unreachable!()
    }

    /// `NextResponse.redirect(url)`: a 307 to `url`.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.redirect")]
    pub fn redirect(url: impl IntoUrl) -> &'static NextResponse {
        unreachable!()
    }

    /// `NextResponse.redirect(url, status)`.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.redirect")]
    pub fn redirect_with_status(url: impl IntoUrl, status: u16) -> &'static NextResponse {
        unreachable!()
    }

    /// `NextResponse.redirect(url, init)`.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.redirect")]
    pub fn redirect_with_init(url: impl IntoUrl, init: ResponseInit<'_>) -> &'static NextResponse {
        unreachable!()
    }

    /// `NextResponse.rewrite(destination)`: `destination`'s response, the
    /// browser's URL kept.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.rewrite")]
    pub fn rewrite(destination: impl IntoUrl) -> &'static NextResponse {
        unreachable!()
    }

    /// `NextResponse.rewrite(destination, init)`.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.rewrite")]
    pub fn rewrite_with_init(destination: impl IntoUrl, init: MiddlewareResponseInit<'_>) -> &'static NextResponse {
        unreachable!()
    }

    /// `NextResponse.next()`: a proxy's going on to the route.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.next")]
    pub fn next() -> &'static NextResponse {
        unreachable!()
    }

    /// `NextResponse.next(init)`, its request's headers `init`'s.
    #[cfg_attr(rust_js, rust_js::link_name = "next/server#NextResponse.next")]
    pub fn next_with_init(init: MiddlewareResponseInit<'_>) -> &'static NextResponse {
        unreachable!()
    }
}

/// What a `string | NextURL | URL` parameter takes: each as it is
/// (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a `string | NextURL | URL`")]
#[cfg_attr(rust_js, rust_js::types = "string | import(\"next/dist/server/web/next-url\").NextURL | URL")]
pub trait IntoUrl: sealed::Sealed {}
impl IntoUrl for &str {}
impl IntoUrl for &String {}
impl IntoUrl for &NextURL {}
impl IntoUrl for &URL {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &String {}
    impl Sealed for &super::NextURL {}
    impl Sealed for &super::URL {}
}

/// A [`next_response::rewrite`]'s or [`next_response::next`]'s init: a
/// `ResponseInit`'s, and the headers the request goes on with.
#[derive(Default)]
pub struct MiddlewareResponseInit<'a> {
    pub status: Option<u16>,
    #[cfg_attr(rust_js, rust_js::name = "statusText")]
    pub status_text: Option<&'a str>,
    pub headers: Option<&'a [&'a [&'a str]]>,
    pub request: Option<ModifiedRequest<'a>>,
}

/// The request a proxy goes on with: its headers.
#[derive(Default)]
pub struct ModifiedRequest<'a> {
    pub headers: Option<&'a Headers>,
}

/// [`NextRequest::next_url`]: a `URL`, its base path and locale apart.
pub struct NextURL(PhantomData<JsObject>);

impl NextURL {
    /// The build's id, of a data request.
    #[cfg_attr(rust_js, rust_js::link_name = "get buildId")]
    pub fn build_id(&self) -> Option<String> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set buildId")]
    pub fn set_build_id(&self, value: Option<&str>) {
        unreachable!()
    }

    /// Its locale, of i18n routing.
    #[cfg_attr(rust_js, rust_js::link_name = "get locale")]
    pub fn locale(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set locale")]
    pub fn set_locale(&self, value: &str) {
        unreachable!()
    }

    /// The default locale, of i18n routing.
    #[cfg_attr(rust_js, rust_js::link_name = "get defaultLocale")]
    pub fn default_locale(&self) -> Option<String> {
        unreachable!()
    }

    /// Its query's parameters.
    #[cfg_attr(rust_js, rust_js::link_name = "get searchParams")]
    pub fn search_params(&self) -> &'static URLSearchParams {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get host")]
    pub fn host(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set host")]
    pub fn set_host(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get hostname")]
    pub fn hostname(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set hostname")]
    pub fn set_hostname(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get port")]
    pub fn port(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set port")]
    pub fn set_port(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get protocol")]
    pub fn protocol(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set protocol")]
    pub fn set_protocol(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get href")]
    pub fn href(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set href")]
    pub fn set_href(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get origin")]
    pub fn origin(&self) -> String {
        unreachable!()
    }

    /// Its path, without the base path or the locale.
    #[cfg_attr(rust_js, rust_js::link_name = "get pathname")]
    pub fn pathname(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set pathname")]
    pub fn set_pathname(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get hash")]
    pub fn hash(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set hash")]
    pub fn set_hash(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get search")]
    pub fn search(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set search")]
    pub fn set_search(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get password")]
    pub fn password(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set password")]
    pub fn set_password(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get username")]
    pub fn username(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set username")]
    pub fn set_username(&self, value: &str) {
        unreachable!()
    }

    /// The app's base path, `basePath` of its config.
    #[cfg_attr(rust_js, rust_js::link_name = "get basePath")]
    pub fn base_path(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set basePath")]
    pub fn set_base_path(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "toJSON")]
    pub fn to_json(&self) -> String {
        unreachable!()
    }

    /// Another of the same URL, which changes apart from this.
    #[cfg_attr(rust_js, rust_js::link_name = "clone")]
    pub fn clone(&self) -> &'static NextURL {
        unreachable!()
    }
}

/// A request's cookies, as `@edge-runtime/cookies`' `RequestCookies`:
/// [`NextRequest::cookies`].
pub struct RequestCookies(PhantomData<JsObject>);

impl RequestCookies {
    /// How many cookies there are.
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

    /// Set the cookie `key` to `value`, in the request.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set(&self, key: &str, value: &str) -> &Self {
        unreachable!()
    }

    /// Set the cookie `options` is.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set_cookie(&self, options: RequestCookie<'_>) -> &Self {
        unreachable!()
    }

    /// Delete the cookie `names`, whether there was one.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    pub fn delete(&self, names: &str) -> bool {
        unreachable!()
    }

    /// Delete the cookies `names`, whether there was each.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    pub fn delete_each(&self, names: &[&str]) -> Vec<bool> {
        unreachable!()
    }

    /// Delete every cookie.
    #[cfg_attr(rust_js, rust_js::link_name = "clear")]
    pub fn clear(&self) -> &Self {
        unreachable!()
    }

    /// The cookies as a `Cookie` header has them, `a=1; b=2`.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }
}

/// A response's cookies, its `Set-Cookie` headers, as `@edge-runtime/cookies`'
/// `ResponseCookies`: [`NextResponse::cookies`].
pub struct ResponseCookies(PhantomData<JsObject>);

impl ResponseCookies {
    /// The cookie named `key`, `None` where there's none.
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    pub fn get(&self, key: &str) -> Option<ResponseCookie<'static>> {
        unreachable!()
    }

    /// Each cookie.
    #[cfg_attr(rust_js, rust_js::link_name = "getAll")]
    pub fn get_all(&self) -> Vec<ResponseCookie<'static>> {
        unreachable!()
    }

    /// Each cookie named `key`.
    #[cfg_attr(rust_js, rust_js::link_name = "getAll")]
    pub fn get_all_with_name(&self, key: &str) -> Vec<ResponseCookie<'static>> {
        unreachable!()
    }

    /// Whether there's a cookie named `name`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    pub fn has(&self, name: &str) -> bool {
        unreachable!()
    }

    /// Set the cookie `key` to `value`.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set(&self, key: &str, value: &str) -> &Self {
        unreachable!()
    }

    /// Set the cookie `key` to `value`, of `cookie`'s options.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set_with_options(&self, key: &str, value: &str, cookie: CookieOptions<'_>) -> &Self {
        unreachable!()
    }

    /// Set the cookie, `options` its name, value and options.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set_cookie(&self, options: ResponseCookie<'_>) -> &Self {
        unreachable!()
    }

    /// Delete the cookie `key`, by expiring it.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    pub fn delete(&self, key: &str) -> &Self {
        unreachable!()
    }

    /// Delete the cookie `options` names, of its domain and path.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    pub fn delete_cookie(&self, options: DeletedCookie<'_>) -> &Self {
        unreachable!()
    }

    /// The cookies as `Set-Cookie` headers have them.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }
}

/// [`NextFetchEvent`](https://nextjs.org/docs/app/api-reference/file-conventions/proxy):
/// what a proxy is given beside its request.
pub struct NextFetchEvent(PhantomData<JsObject>);

impl NextFetchEvent {
    /// The page it's for.
    #[cfg_attr(rust_js, rust_js::link_name = "get sourcePage")]
    pub fn source_page(&self) -> String {
        unreachable!()
    }

    /// Keep the proxy going until `promise` settles, after its response.
    #[cfg_attr(rust_js, rust_js::link_name = "waitUntil")]
    pub fn wait_until<T>(&self, promise: Promise<T>) {
        unreachable!()
    }

    /// Where the proxy throws, go on to the route as if it hadn't run.
    #[cfg_attr(rust_js, rust_js::link_name = "passThroughOnException")]
    pub fn pass_through_on_exception(&self) {
        unreachable!()
    }
}

/// What a proxy returns: a response, `NextResponse`'s or another, or
/// `None`, to go on to the route.
pub type NextMiddlewareResult = Option<&'static Response>;

/// A proxy, as `NextMiddleware` types one: a function of its request, and
/// its [`NextFetchEvent`] too, giving a [`NextMiddlewareResult`], or
/// `async`, a future of one. `M` only tells them apart.
pub trait NextMiddleware<M> {}

#[doc(hidden)]
pub struct Returned<const EVENT: bool>;
#[doc(hidden)]
pub struct Awaited<const EVENT: bool>;

impl<F: Fn(&'static NextRequest) -> NextMiddlewareResult> NextMiddleware<Returned<false>> for F {}
impl<F: Fn(&'static NextRequest, &'static NextFetchEvent) -> NextMiddlewareResult> NextMiddleware<Returned<true>> for F {}
impl<F: Fn(&'static NextRequest) -> R, R: Future<Output = NextMiddlewareResult>> NextMiddleware<Awaited<false>> for F {}
impl<F: Fn(&'static NextRequest, &'static NextFetchEvent) -> R, R: Future<Output = NextMiddlewareResult>> NextMiddleware<Awaited<true>>
    for F
{
}

/// A proxy, `NextMiddleware`'s name since Next.js 16.
pub use NextMiddleware as NextProxy;

/// A proxy's `config`, as `MiddlewareConfig` types it: the paths it runs
/// for. Its literal is what Next.js reads, as it builds the app.
#[derive(Default)]
pub struct MiddlewareConfig<'a> {
    /// The paths, `"/about/:path*"`, or each of a list.
    pub matcher: Option<Matcher<'a>>,
    /// The regions it runs in, of an edge deployment.
    pub regions: Option<OneOrMany<'a, &'a str>>,
    /// Files whose dynamic code it may run, by globs.
    #[cfg_attr(rust_js, rust_js::name = "unstable_allowDynamic")]
    pub unstable_allow_dynamic: Option<OneOrMany<'a, &'a str>>,
}

/// A proxy's `config`, `MiddlewareConfig`'s name since Next.js 16.
pub type ProxyConfig<'a> = MiddlewareConfig<'a>;

/// A [`MiddlewareConfig`]'s `matcher`: a path, or a list of paths and
/// routes, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Matcher<'a> {
    Path(&'a str),
    List(&'a [MatcherItem<'a>]),
}

/// One of a [`Matcher`]'s list: a path, or a route of its conditions.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum MatcherItem<'a> {
    Path(&'a str),
    Route(RouteMatcher<'a>),
}

/// A path the proxy runs for, where its request has what `has` says and
/// hasn't what `missing` does.
pub struct RouteMatcher<'a> {
    pub source: &'a str,
    /// `Some(false)`: the path without the locale's prefix.
    pub locale: Option<bool>,
    pub has: Option<&'a [RouteHas<'a>]>,
    pub missing: Option<&'a [RouteHas<'a>]>,
}

/// A request's header, query, cookie or host, by `type`, and its `key` and
/// `value`, as Next.js's `RouteHas` types it: a host's of a `value`, no
/// `key`.
pub struct RouteHas<'a> {
    /// `"header"`, `"query"`, `"cookie"` or `"host"`.
    pub r#type: &'a str,
    pub key: Option<&'a str>,
    pub value: Option<&'a str>,
}


/// The request's browser, device and system, from its `User-Agent`, as
/// `userAgent(request)`.
#[cfg_attr(rust_js, rust_js::link_name = "next/server#userAgent")]
pub fn user_agent(request: &Request) -> UserAgent {
    unreachable!()
}

/// [`user_agent`] of a `User-Agent`'s text.
#[cfg_attr(rust_js, rust_js::link_name = "next/server#userAgentFromString")]
pub fn user_agent_from_string(input: Option<&str>) -> UserAgent {
    unreachable!()
}

/// What [`user_agent`] gives.
pub struct UserAgent {
    #[cfg_attr(rust_js, rust_js::name = "isBot")]
    pub is_bot: bool,
    /// The `User-Agent` itself.
    pub ua: String,
    pub browser: UserAgentBrowser,
    pub device: UserAgentDevice,
    pub engine: UserAgentEngine,
    pub os: UserAgentOs,
    pub cpu: UserAgentCpu,
}

/// A [`UserAgent`]'s browser.
pub struct UserAgentBrowser {
    pub name: Option<String>,
    pub version: Option<String>,
    pub major: Option<String>,
}

/// A [`UserAgent`]'s device: its `type`, `"mobile"`, `"tablet"`.
pub struct UserAgentDevice {
    pub model: Option<String>,
    pub r#type: Option<String>,
    pub vendor: Option<String>,
}

/// A [`UserAgent`]'s browser engine.
pub struct UserAgentEngine {
    pub name: Option<String>,
    pub version: Option<String>,
}

/// A [`UserAgent`]'s operating system.
pub struct UserAgentOs {
    pub name: Option<String>,
    pub version: Option<String>,
}

/// A [`UserAgent`]'s processor.
pub struct UserAgentCpu {
    pub architecture: Option<String>,
}

/// Run `task` after the response is sent, as `after(task)`: a function,
/// `async` or not, or a promise.
#[cfg_attr(rust_js, rust_js::link_name = "next/server#after")]
pub fn after<M>(task: impl AfterTask<M>) {
    unreachable!()
}

/// What [`after`] takes: a function, or a promise. `M` only tells them
/// apart.
pub trait AfterTask<M> {}

#[doc(hidden)]
pub struct Called;
#[doc(hidden)]
pub struct Settled;

impl<T, F: FnOnce() -> T + 'static> AfterTask<Called> for F {}
impl<T> AfterTask<Settled> for Promise<T> {}

/// Wait for a request, as `await connection()`: what's rendered after it
/// is the request's, not the build's.
#[cfg_attr(rust_js, rust_js::link_name = "next/server#connection")]
pub fn connection() -> Promise<()> {
    unreachable!()
}

/// `URLPattern`, Next.js's, for a runtime without one.
pub mod url_pattern {
    use super::*;

    /// `new URLPattern()`, of any URL.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/server#URLPattern")]
    pub fn new() -> &'static URLPattern {
        unreachable!()
    }

    /// `new URLPattern(input)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/server#URLPattern")]
    pub fn new_with_input(input: impl IntoURLPatternInput) -> &'static URLPattern {
        unreachable!()
    }

    /// `new URLPattern(input, baseURL)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new next/server#URLPattern")]
    pub fn new_with_base_url(input: impl IntoURLPatternInput, base_url: &str) -> &'static URLPattern {
        unreachable!()
    }
}

/// `ImageResponse`, which moved to `next/og`: it throws.
#[deprecated = "`ImageResponse` is `next::og::ImageResponse`"]
#[cfg_attr(rust_js, rust_js::link_name = "next/server#ImageResponse")]
pub fn ImageResponse() -> ! {
    unreachable!()
}
