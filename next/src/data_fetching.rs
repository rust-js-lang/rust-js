//! The Pages Router's [data fetching](https://nextjs.org/docs/pages/building-your-application/data-fetching),
//! as `next` types it: what a page's `getStaticProps` and `getStaticPaths`
//! are given and give, each an `async fn` the page exports, as react.dev's
//! errors page has them.
//!
//! ```rust,ignore
//! pub async fn getStaticProps(GetStaticPropsContext { params, .. }: GetStaticPropsContext<Params>) -> GetStaticPropsResult<Props> {
//!     GetStaticPropsResult::Props(StaticProps { props: Props { .. } })
//! }
//! ```
//!
//! What's not here yet: a result's `revalidate`, which a `None` would give
//! as `revalidate: undefined`, a `redirect`, and a fallback of `"blocking"`.

use js::Unknown;

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
/// the page's props, or that it isn't found, each its object.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum GetStaticPropsResult<Props> {
    Props(StaticProps<Props>),
    NotFound(StaticNotFound),
}

/// `{ props }`: the props the page is rendered with.
pub struct StaticProps<Props> {
    pub props: Props,
}

/// `{ notFound: true }`: the page's 404.
pub struct StaticNotFound {
    /// Always `true`.
    #[cfg_attr(rust_js, rust_js::name = "notFound")]
    pub not_found: bool,
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
    pub fallback: bool,
}

/// A path to build: written out, `"/errors/1"`, or the route's parameters.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum StaticPath<Params> {
    Path(String),
    Params(StaticPathParams<Params>),
}

/// `{ params }`: a path by the route's parameters.
pub struct StaticPathParams<Params> {
    pub params: Params,
}
