//! [A custom server](https://nextjs.org/docs/pages/guides/custom-server):
//! `next`'s default export, Next.js's server made by a program of its own,
//! which hands it each request.

use core::marker::PhantomData;

use js::{JsObject, Promise, Unknown};
use node::http::{IncomingMessage, ServerResponse};

use crate::NextConfig;

/// `next(options)`: Next.js's server, to `prepare` and hand requests to, as
/// `createServer` types it.
#[cfg_attr(rust_js, rust_js::link_name = "next#default")]
pub fn next(options: NextServerOptions<'_>) -> &'static NextServer {
    unreachable!()
}

/// How [`next`] makes its server, as `NextServerOptions` and
/// `NextBundlerOptions` type it.
#[derive(Default)]
pub struct NextServerOptions<'a> {
    /// Its development server, of Fast Refresh and checks.
    pub dev: Option<bool>,
    /// The app's directory: `"."`.
    pub dir: Option<&'a str>,
    /// Without its logs.
    pub quiet: Option<bool>,
    pub hostname: Option<&'a str>,
    pub port: Option<f64>,
    /// Made by a custom server, as it is: `true`.
    #[cfg_attr(rust_js, rust_js::name = "customServer")]
    pub custom_server: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "minimalMode")]
    pub minimal_mode: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "experimentalTestProxy")]
    pub experimental_test_proxy: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "experimentalHttpsServer")]
    pub experimental_https_server: Option<bool>,
    /// Its config, in place of `next.config.js`'s.
    pub conf: Option<NextConfig<'a>>,
    /// Bundled by Turbopack, its default.
    pub turbopack: Option<bool>,
    /// What `turbopack` was named.
    pub turbo: Option<bool>,
    /// Bundled by webpack.
    pub webpack: Option<bool>,
}

/// What a [`NextServer`] hands a request to: Next.js's handling of it.
pub type RequestHandler = &'static dyn Fn(&IncomingMessage, &ServerResponse) -> Promise<()>;

/// Next.js's server, as a custom server has it.
pub struct NextServer(PhantomData<JsObject>);

impl NextServer {
    #[cfg_attr(rust_js, rust_js::link_name = "get hostname")]
    pub fn hostname(&self) -> Option<&'static str> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get port")]
    pub fn port(&self) -> Option<f64> {
        unreachable!()
    }

    /// Get it ready, before it's handed a request.
    #[cfg_attr(rust_js, rust_js::link_name = "prepare")]
    pub fn prepare(&self) -> Promise<()> {
        unreachable!()
    }

    /// What handles each request, `handle(req, res)`.
    #[cfg_attr(rust_js, rust_js::link_name = "getRequestHandler")]
    pub fn get_request_handler(&self) -> RequestHandler {
        unreachable!()
    }

    /// Serve its assets from `asset_prefix`, a CDN's URL.
    #[cfg_attr(rust_js, rust_js::link_name = "setAssetPrefix")]
    pub fn set_asset_prefix(&self, asset_prefix: &str) {
        unreachable!()
    }

    /// Render `pathname`'s page, of `query`, as the response.
    #[cfg_attr(rust_js, rust_js::link_name = "render")]
    pub fn render(&self, req: &IncomingMessage, res: &ServerResponse, pathname: &str) -> Promise<()> {
        unreachable!()
    }

    /// `pathname`'s page as HTML, `None` where it isn't one.
    #[cfg_attr(rust_js, rust_js::link_name = "renderToHTML")]
    pub fn render_to_html(&self, req: &IncomingMessage, res: &ServerResponse, pathname: &str) -> Promise<Option<String>> {
        unreachable!()
    }

    /// Render the error page of `err` as the response.
    #[cfg_attr(rust_js, rust_js::link_name = "renderError")]
    pub fn render_error(&self, err: Option<&Unknown>, req: &IncomingMessage, res: &ServerResponse, pathname: &str) -> Promise<()> {
        unreachable!()
    }

    /// The error page of `err` as HTML.
    #[cfg_attr(rust_js, rust_js::link_name = "renderErrorToHTML")]
    pub fn render_error_to_html(&self, err: Option<&Unknown>, req: &IncomingMessage, res: &ServerResponse, pathname: &str) -> Promise<Option<String>> {
        unreachable!()
    }

    /// Render the 404 page as the response.
    #[cfg_attr(rust_js, rust_js::link_name = "render404")]
    pub fn render_404(&self, req: &IncomingMessage, res: &ServerResponse) -> Promise<()> {
        unreachable!()
    }

    /// Log `err` as Next.js logs its own.
    #[cfg_attr(rust_js, rust_js::link_name = "logError")]
    pub fn log_error(&self, err: &Unknown) {
        unreachable!()
    }

    /// Render the static page `url_path` again, on demand.
    #[cfg_attr(rust_js, rust_js::link_name = "revalidate")]
    pub fn revalidate(&self, url_path: &str) -> Promise<()> {
        unreachable!()
    }

    /// Stop it.
    #[cfg_attr(rust_js, rust_js::link_name = "close")]
    pub fn close(&self) -> Promise<()> {
        unreachable!()
    }
}
