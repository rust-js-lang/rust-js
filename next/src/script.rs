//! [`next/script`](https://nextjs.org/docs/app/api-reference/components/script):
//! a third party's `<script>`, loaded when its `strategy` says.

use js::Unknown;
use react::attributes::ScriptHTMLAttributes;
use react::{JSX, ReactNode};

/// `<Script src="https://example.com/script.js" strategy={Some("lazyOnload")} />`.
#[cfg_attr(rust_js, rust_js::link_name = "next/script#default")]
pub fn Script<C: ReactNode>(props: ScriptProps<'_, C>) -> JSX::Element {
    unreachable!()
}

/// What a [`Script`] is given, as Next.js types it: a `<script>`'s
/// attributes, `src` among them, and its own; each `None`, which Next.js
/// takes as its default, but what's given.
#[derive(Default)]
pub struct ScriptProps<'a, C> {
    /// The `<script>`'s, as React's `ScriptHTMLAttributes`: `src`,
    /// `async`, `nonce`. Those named here are these.
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub script: ScriptHTMLAttributes<'a>,
    /// When it loads: `"afterInteractive"`, its default,
    /// `"beforeInteractive"`, `"lazyOnload"`, or `"worker"`.
    pub strategy: Option<&'a str>,
    /// Its `id`, which an inline script needs, to be loaded once.
    pub id: Option<&'a str>,
    /// Called as it's loaded, given the `load` event.
    #[cfg_attr(rust_js, rust_js::name = "onLoad")]
    pub on_load: Option<Box<dyn Fn(&Unknown)>>,
    /// Called as it's loaded, and each time its component mounts again.
    #[cfg_attr(rust_js, rust_js::name = "onReady")]
    pub on_ready: Option<Box<dyn Fn()>>,
    /// Called where it fails to load, given the `error` event.
    #[cfg_attr(rust_js, rust_js::name = "onError")]
    pub on_error: Option<Box<dyn Fn(&Unknown)>>,
    /// Stylesheets it needs, loaded with it.
    pub stylesheets: Option<&'a [&'a str]>,
    /// An inline script's text, in place of `src`.
    pub children: C,
}

/// What [`ScriptProps`] were called.
#[deprecated = "Use `ScriptProps` instead."]
pub type Props<'a, C> = ScriptProps<'a, C>;

/// Load a script, as a [`Script`] of `props` does, where it's called.
#[cfg_attr(rust_js, rust_js::link_name = "next/script#handleClientScriptLoad")]
pub fn handle_client_script_load<C: ReactNode>(props: ScriptProps<'_, C>) {
    unreachable!()
}

/// Load each script, as [`handle_client_script_load`] does one.
#[cfg_attr(rust_js, rust_js::link_name = "next/script#initScriptLoader")]
pub fn init_script_loader<C: ReactNode>(script_loader_items: Vec<ScriptProps<'_, C>>) {
    unreachable!()
}
