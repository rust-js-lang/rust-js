//! A route segment's config, what its `page.rs` or `layout.rs` exports as
//! Next.js reads it: its `instant`, and its parameters' matching.

use js::Dict;

use crate::QueryValue;

/// A segment's `instant`, as `Instant` types it: whether its navigation is
/// checked to be instant, or how; each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Instant<'a> {
    Bool(bool),
    Config(InstantConfig<'a>),
}

/// How a segment's navigation is checked to be instant.
#[derive(Default)]
pub struct InstantConfig<'a> {
    /// `"warning"` or `"experimental-error"`.
    pub level: Option<&'a str>,
    /// Requests it's checked of.
    pub unstable_samples: Option<&'a [InstantSample<'a>]>,
    /// The routes it's checked from.
    pub unstable_from: Option<&'a [&'a str]>,
    #[cfg_attr(rust_js, rust_js::name = "unstable_disableValidation")]
    pub unstable_disable_validation: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "unstable_disableDevValidation")]
    pub unstable_disable_dev_validation: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "unstable_disableBuildValidation")]
    pub unstable_disable_build_validation: Option<bool>,
}

/// A request an [`InstantConfig`] is checked of: its cookies, headers,
/// parameters and query.
#[derive(Default)]
pub struct InstantSample<'a> {
    pub cookies: Option<&'a [SampleCookie<'a>]>,
    pub headers: Option<&'a [(&'a str, Option<&'a str>)]>,
    pub params: Option<&'a Dict<QueryValue>>,
    #[cfg_attr(rust_js, rust_js::name = "searchParams")]
    pub search_params: Option<&'a Dict<Option<QueryValue>>>,
}

/// A sample's cookie: its name, and its value or none.
pub struct SampleCookie<'a> {
    pub name: &'a str,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Option<&'a str>,
}

/// How a dynamic segment's parameter is matched, of a path not generated,
/// as `ParamMatchingMode` types it.
pub enum ParamMatchingMode {
    /// A 404.
    #[cfg_attr(rust_js, rust_js::name = "not-found")]
    NotFound,
    /// Rendered on the request, which waits.
    #[cfg_attr(rust_js, rust_js::name = "blocking")]
    Blocking,
    /// Its fallback shell first.
    #[cfg_attr(rust_js, rust_js::name = "fallback")]
    Fallback,
    /// Rendered on each request.
    #[cfg_attr(rust_js, rust_js::name = "dynamic")]
    Dynamic,
}

/// A segment's `unstable_paramMatching`, as `ParamMatching` types it: each
/// parameter's mode by its name. A struct of the route's parameters, each an
/// `Option<ParamMatchingMode>`, is its form of the names, as
/// `ParamMatching<"slug">` is.
pub type ParamMatching<'a> = &'a Dict<ParamMatchingMode>;
