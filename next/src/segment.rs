//! A route segment's config, what its `page.rs` or `layout.rs` exports as
//! Next.js reads it: its `instant`.

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
