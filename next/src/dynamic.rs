//! [`next/dynamic`](https://nextjs.org/docs/app/guides/lazy-loading):
//! a component loaded when it's first rendered, in a `thread_local!`, as
//! Next.js's compiler needs it, at the module's top.

use js::{Dict, Promise, Unknown};
use react::{ComponentValue, JSX, Module};

/// [`dynamic`](https://nextjs.org/docs/app/guides/lazy-loading#nextdynamic),
/// in a `thread_local!`: a component whose module `loader` loads,
/// `dynamic(|| import_module("./hello.jsx"))`.
#[cfg_attr(rust_js, rust_js::link_name = "next/dynamic#default")]
pub fn dynamic<P>(loader: impl Fn() -> LoaderComponent<P> + 'static) -> ComponentValue<P> {
    unreachable!()
}

/// [`dynamic`], of `options`: what it shows while it loads, and whether
/// the server renders it.
#[cfg_attr(rust_js, rust_js::link_name = "next/dynamic#default")]
pub fn dynamic_with_options<P>(loader: impl Fn() -> LoaderComponent<P> + 'static, options: DynamicOptions<P>) -> ComponentValue<P> {
    unreachable!()
}

/// [`dynamic`] of `options` alone, its `loader` among them.
#[cfg_attr(rust_js, rust_js::link_name = "next/dynamic#default")]
pub fn dynamic_of_options<P>(options: DynamicOptions<P>) -> ComponentValue<P> {
    unreachable!()
}

/// `noSSR(LoadableInitializer, loadableOptions)`: what [`dynamic`] makes of
/// `{ ssr: false }`, rendered only in the browser.
#[cfg_attr(rust_js, rust_js::link_name = "next/dynamic#noSSR")]
pub fn no_ssr<P>(loadable_initializer: LoadableFn<P>, loadable_options: DynamicOptions<P>) -> ComponentValue<P> {
    unreachable!()
}

/// What a loader gives: a promise of its module, whose default export is
/// the component, or `Module::of` the one it names.
pub type LoaderComponent<P> = Promise<Module<P>>;

/// What loads a component's module.
pub type Loader<P> = Box<dyn Fn() -> LoaderComponent<P>>;

/// Each module's loader, by its path, as Next.js's compiler writes it.
pub type LoaderMap = Dict<Box<dyn Fn() -> Loader<Unknown>>>;

/// What Next.js's compiler adds to a [`dynamic`]'s options: the modules it
/// loads, for the server to preload.
#[derive(Default)]
pub struct LoadableGeneratedOptions {
    pub webpack: Option<Box<dyn Fn() -> &'static Unknown>>,
    pub modules: Option<Box<dyn Fn() -> LoaderMap>>,
}

/// What [`DynamicOptions::loading`] is given.
pub struct DynamicOptionsLoadingProps {
    /// What loading it threw.
    pub error: Option<&'static Unknown>,
    #[cfg_attr(rust_js, rust_js::name = "isLoading")]
    pub is_loading: Option<bool>,
    /// Whether it's loaded longer than its delay.
    #[cfg_attr(rust_js, rust_js::name = "pastDelay")]
    pub past_delay: Option<bool>,
    /// Load it again.
    pub retry: Option<Box<dyn Fn()>>,
    #[cfg_attr(rust_js, rust_js::name = "timedOut")]
    pub timed_out: Option<bool>,
}

/// How a [`dynamic`] component loads, as `DynamicOptions` types it.
pub struct DynamicOptions<P> {
    /// What it shows while it loads.
    pub loading: Option<Box<dyn Fn(DynamicOptionsLoadingProps) -> JSX::Element>>,
    /// What loads it, of [`dynamic_of_options`].
    pub loader: Option<Loader<P>>,
    #[cfg_attr(rust_js, rust_js::name = "loadableGenerated")]
    pub loadable_generated: Option<LoadableGeneratedOptions>,
    /// Rendered on the server too: `true`. Only a client component's may be
    /// `Some(false)`.
    pub ssr: Option<bool>,
    pub webpack: Option<Box<dyn Fn() -> &'static Unknown>>,
    pub modules: Option<Box<dyn Fn() -> LoaderMap>>,
}

impl<P> Default for DynamicOptions<P> {
    fn default() -> Self {
        DynamicOptions { loading: None, loader: None, loadable_generated: None, ssr: None, webpack: None, modules: None }
    }
}

/// [`DynamicOptions`], as Next.js's loader calls them.
pub type LoadableOptions<P> = DynamicOptions<P>;

/// What makes a component of its [`LoadableOptions`].
pub type LoadableFn<P> = Box<dyn Fn(LoadableOptions<P>) -> ComponentValue<P>>;

/// A component [`dynamic`] makes.
pub type LoadableComponent<P> = ComponentValue<P>;
