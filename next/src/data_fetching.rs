//! The Pages Router's [data fetching](https://nextjs.org/docs/pages/building-your-application/data-fetching),
//! as `next` types it: what a page's `getStaticProps`, `getStaticPaths`
//! and `getServerSideProps` are given and give, each an `async fn` the page
//! exports, as react.dev's errors page has them.
//!
//! ```rust,ignore
//! pub async fn getStaticProps(GetStaticPropsContext { params, .. }: GetStaticPropsContext<Params>) -> GetStaticPropsResult<Props> {
//!     GetStaticPropsResult::Props(StaticProps { props: Props { .. }, revalidate: None })
//! }
//! ```

use js::{Dict, Unknown};
use node::http::{IncomingMessage, ServerResponse};

/// [`GetStaticPropsContext`](https://nextjs.org/docs/pages/api-reference/functions/get-static-props#context-parameter):
/// what `getStaticProps` is given, the route's `Params` among it.
pub struct GetStaticPropsContext<Params> {
    /// The dynamic route's parameters; `None` for a page that isn't one.
    pub params: Option<Params>,
    pub preview: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "previewData")]
    pub preview_data: Option<&'static Unknown>,
    #[cfg_attr(rust_js, rust_js::name = "draftMode")]
    pub draft_mode: Option<bool>,
    pub locale: Option<String>,
    pub locales: Option<Vec<String>>,
    #[cfg_attr(rust_js, rust_js::name = "defaultLocale")]
    pub default_locale: Option<String>,
    #[cfg_attr(rust_js, rust_js::name = "revalidateReason")]
    pub revalidate_reason: Option<RevalidateReason>,
}

/// Why `getStaticProps` runs.
pub enum RevalidateReason {
    #[cfg_attr(rust_js, rust_js::name = "on-demand")]
    OnDemand,
    #[cfg_attr(rust_js, rust_js::name = "build")]
    Build,
    #[cfg_attr(rust_js, rust_js::name = "stale")]
    Stale,
}

/// [`GetStaticPropsResult`](https://nextjs.org/docs/pages/api-reference/functions/get-static-props#getstaticprops-return-values):
/// the page's props, a redirect, or that it isn't found, each its object.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum GetStaticPropsResult<Props> {
    Props(StaticProps<Props>),
    Redirect(StaticRedirect),
    NotFound(StaticNotFound),
}

/// `{ props, revalidate }`: the props the page is rendered with, and when
/// it's rendered again.
pub struct StaticProps<Props> {
    pub props: Props,
    pub revalidate: Option<Revalidate>,
}

/// `{ redirect, revalidate }`: where the page goes instead.
pub struct StaticRedirect {
    pub redirect: Redirect<'static>,
    pub revalidate: Option<Revalidate>,
}

/// `{ notFound: true, revalidate }`: the page's 404.
pub struct StaticNotFound {
    /// Always `true`.
    #[cfg_attr(rust_js, rust_js::name = "notFound")]
    pub not_found: bool,
    pub revalidate: Option<Revalidate>,
}

/// When a static page is rendered again, `number | boolean`: after its
/// seconds, or `false`, never, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Revalidate {
    Seconds(f64),
    Bool(bool),
}

/// [`Redirect`](https://nextjs.org/docs/pages/api-reference/functions/get-static-props#redirect):
/// where a page goes instead, by its status or for good.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Redirect<'a> {
    Status(StatusRedirect<'a>),
    Permanent(PermanentRedirect<'a>),
}

/// A redirect of its status: `301`, `302`, `303`, `307` or `308`.
pub struct StatusRedirect<'a> {
    #[cfg_attr(rust_js, rust_js::name = "statusCode")]
    pub status_code: u16,
    pub destination: &'a str,
    /// `Some(false)`: the destination without the app's base path.
    #[cfg_attr(rust_js, rust_js::name = "basePath")]
    pub base_path: Option<bool>,
}

/// A redirect for good, a 308, or not, a 307.
pub struct PermanentRedirect<'a> {
    pub permanent: bool,
    pub destination: &'a str,
    #[cfg_attr(rust_js, rust_js::name = "basePath")]
    pub base_path: Option<bool>,
}

/// [`GetStaticPathsContext`](https://nextjs.org/docs/pages/api-reference/functions/get-static-paths):
/// what `getStaticPaths` is given.
pub struct GetStaticPathsContext {
    pub locales: Option<Vec<String>>,
    #[cfg_attr(rust_js, rust_js::name = "defaultLocale")]
    pub default_locale: Option<String>,
}

/// [`GetStaticPathsResult`](https://nextjs.org/docs/pages/api-reference/functions/get-static-paths#getstaticpaths-return-values):
/// the paths a dynamic route is built for, and whether another is rendered
/// when asked for.
pub struct GetStaticPathsResult<Params> {
    pub paths: Vec<StaticPath<Params>>,
    pub fallback: GetStaticPathsFallback,
}

/// A [`GetStaticPathsResult`]'s `fallback`, `boolean | "blocking"`: `true`,
/// `false`, or `"blocking"`, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum GetStaticPathsFallback {
    Bool(bool),
    /// `"blocking"`: a path not built is rendered as it's asked for.
    Str(&'static str),
}

impl From<bool> for GetStaticPathsFallback {
    fn from(value: bool) -> Self {
        GetStaticPathsFallback::Bool(value)
    }
}

/// A path to build: written out, `"/errors/1"`, or the route's parameters.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum StaticPath<Params> {
    Path(String),
    Params(StaticPathParams<Params>),
}

/// `{ params, locale }`: a path by the route's parameters, and its locale.
pub struct StaticPathParams<Params> {
    pub params: Params,
    pub locale: Option<String>,
}

/// [`GetServerSidePropsContext`](https://nextjs.org/docs/pages/api-reference/functions/get-server-side-props#context-parameter):
/// what `getServerSideProps` is given, its request and response among it.
pub struct GetServerSidePropsContext<Params> {
    pub req: &'static IncomingMessage,
    pub res: &'static ServerResponse,
    /// The dynamic route's parameters; `None` for a page that isn't one.
    pub params: Option<Params>,
    /// The URL's query, each parameter's value or values.
    pub query: &'static Dict<Unknown>,
    pub preview: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "previewData")]
    pub preview_data: Option<&'static Unknown>,
    #[cfg_attr(rust_js, rust_js::name = "draftMode")]
    pub draft_mode: Option<bool>,
    /// The URL the page is rendered for, without the locale's prefix.
    #[cfg_attr(rust_js, rust_js::name = "resolvedUrl")]
    pub resolved_url: String,
    pub locale: Option<String>,
    pub locales: Option<Vec<String>>,
    #[cfg_attr(rust_js, rust_js::name = "defaultLocale")]
    pub default_locale: Option<String>,
}

/// [`GetServerSidePropsResult`](https://nextjs.org/docs/pages/api-reference/functions/get-server-side-props#getserversideprops-return-values):
/// the page's props, a redirect, or that it isn't found, each its object.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum GetServerSidePropsResult<Props> {
    Props(ServerProps<Props>),
    Redirect(ServerRedirect),
    NotFound(ServerNotFound),
}

/// `{ props }`: the props the page is rendered with.
pub struct ServerProps<Props> {
    pub props: Props,
}

/// `{ redirect }`: where the page goes instead.
pub struct ServerRedirect {
    pub redirect: Redirect<'static>,
}

/// `{ notFound: true }`: the page's 404.
pub struct ServerNotFound {
    /// Always `true`.
    #[cfg_attr(rust_js, rust_js::name = "notFound")]
    pub not_found: bool,
}

/// What a page's `getStaticProps` is, as `GetStaticProps` types it: an
/// `async fn` of its [`GetStaticPropsContext`], giving its
/// [`GetStaticPropsResult`].
pub trait GetStaticProps<Props, Params> {}

impl<F, R, Props, Params> GetStaticProps<Props, Params> for F
where
    F: Fn(GetStaticPropsContext<Params>) -> R,
    R: Future<Output = GetStaticPropsResult<Props>>,
{
}

/// What a page's `getStaticPaths` is, as `GetStaticPaths` types it.
pub trait GetStaticPaths<Params> {}

impl<F, R, Params> GetStaticPaths<Params> for F
where
    F: Fn(GetStaticPathsContext) -> R,
    R: Future<Output = GetStaticPathsResult<Params>>,
{
}

/// What a page's `getServerSideProps` is, as `GetServerSideProps` types it.
pub trait GetServerSideProps<Props, Params> {}

impl<F, R, Props, Params> GetServerSideProps<Props, Params> for F
where
    F: Fn(GetServerSidePropsContext<Params>) -> R,
    R: Future<Output = GetServerSidePropsResult<Props>>,
{
}
