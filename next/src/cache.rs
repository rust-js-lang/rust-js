//! [`next/cache`](https://nextjs.org/docs/app/api-reference/functions/cacheLife):
//! how long what a `"use cache"` module renders is kept, and making it stale.

use js::Promise;

/// Keep what this `"use cache"` renders for `profile`'s time: a profile's
/// name, `"default"`, `"seconds"`, `"minutes"`, `"hours"`, `"days"`,
/// `"weeks"`, `"max"` or one of the config's, or a [`CacheLife`] of its own.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#cacheLife")]
pub fn cache_life(profile: impl IntoCacheLife) {
    unreachable!()
}

/// [`cache_life`], by its name before Next.js 16.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#unstable_cacheLife")]
pub fn unstable_cache_life(profile: impl IntoCacheLife) {
    unreachable!()
}

/// How long a cached value lasts, in seconds, as `CacheLife` types it.
#[derive(Default)]
pub struct CacheLife {
    /// How long a client uses it without asking the server.
    pub stale: Option<f64>,
    /// How long until the server makes it again, in the background.
    pub revalidate: Option<f64>,
    /// How long, without a request, until it's gone.
    pub expire: Option<f64>,
}

/// What a `CacheLifeProfiles | CacheLife` parameter takes: a profile's
/// name, or a [`CacheLife`], each as it is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a cache life's profile, a `&str`, nor a `CacheLife`")]
#[cfg_attr(rust_js, rust_js::types = "string | import(\"next/dist/server/use-cache/cache-life\").CacheLife")]
pub trait IntoCacheLife: sealed::Sealed {}
impl IntoCacheLife for &str {}
impl IntoCacheLife for CacheLife {}

/// Tag what this `"use cache"` renders with `tags`, which
/// [`revalidate_tag`] and [`update_tag`] make stale.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#cacheTag")]
#[cfg_attr(rust_js, rust_js::variadic)]
pub fn cache_tag(tags: &[&str]) {
    unreachable!()
}

/// [`cache_tag`], by its name before Next.js 16.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#unstable_cacheTag")]
#[cfg_attr(rust_js, rust_js::variadic)]
pub fn unstable_cache_tag(tags: &[&str]) {
    unreachable!()
}

/// Make what's cached for `original_path` stale, to be made again on its
/// next request.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#revalidatePath")]
pub fn revalidate_path(original_path: &str) {
    unreachable!()
}

/// [`revalidate_path`], of its `"layout"` and what's below it, or its
/// `"page"`, where the path has a dynamic segment.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#revalidatePath")]
pub fn revalidate_path_with_type(original_path: &str, r#type: &str) {
    unreachable!()
}

/// Make what's cached with `tag` stale, of `profile`'s life: a profile's
/// name, `"max"`, or a [`CacheLifeConfig`].
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#revalidateTag")]
pub fn revalidate_tag(tag: &str, profile: impl IntoRevalidateProfile) {
    unreachable!()
}

/// How long a stale value of [`revalidate_tag`]'s lasts, in seconds.
#[derive(Default)]
pub struct CacheLifeConfig {
    pub expire: Option<f64>,
}

/// What a `string | CacheLifeConfig` parameter takes: each as it is
/// (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a cache life's profile, a `&str`, nor a `CacheLifeConfig`")]
#[cfg_attr(rust_js, rust_js::types = "string | { expire?: number }")]
pub trait IntoRevalidateProfile: sealed::Sealed {}
impl IntoRevalidateProfile for &str {}
impl IntoRevalidateProfile for CacheLifeConfig {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for super::CacheLife {}
    impl Sealed for super::CacheLifeConfig {}
}

/// Make what's cached with `tag` stale and made again at once, in a Server
/// Action, so its result shows it.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#updateTag")]
pub fn update_tag(tag: &str) {
    unreachable!()
}

/// Render the client's route again, in a Server Action.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#refresh")]
pub fn refresh() {
    unreachable!()
}

/// Keep what's rendered from here out of any cache, as `unstable_noStore()`.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#unstable_noStore")]
pub fn unstable_no_store() {
    unreachable!()
}

/// Wait for the request, as `await io()` does, before reading what changes
/// each time, the time or a random number.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#io")]
pub fn io() -> Promise<()> {
    unreachable!()
}

/// `cb`, its results kept across requests by its arguments and
/// `key_parts`, as `unstable_cache(cb, keyParts, options)`: an async
/// function.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#unstable_cache")]
pub fn unstable_cache<F>(cb: F) -> F {
    unreachable!()
}

/// [`unstable_cache`], its key `key_parts` too.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#unstable_cache")]
pub fn unstable_cache_with_key_parts<F>(cb: F, key_parts: &[&str]) -> F {
    unreachable!()
}

/// [`unstable_cache`], of `options`.
#[cfg_attr(rust_js, rust_js::link_name = "next/cache#unstable_cache")]
pub fn unstable_cache_with_options<F>(cb: F, key_parts: Option<&[&str]>, options: UnstableCacheOptions<'_>) -> F {
    unreachable!()
}

/// How long [`unstable_cache`] keeps a result, and the tags it's of.
#[derive(Default)]
pub struct UnstableCacheOptions<'a> {
    /// Seconds, or `Revalidate::Never(false)`, for good.
    pub revalidate: Option<Revalidate>,
    pub tags: Option<&'a [&'a str]>,
}

/// `number | false`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Revalidate {
    Seconds(f64),
    /// `false`: kept for good.
    Never(bool),
}
