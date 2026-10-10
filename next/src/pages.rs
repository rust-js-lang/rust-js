//! The Pages Router's pages and API routes, as `next` types them: a page,
//! what its `getInitialProps` is given, and an API route's request and
//! response, `pages/api/hello.rs`'s `pub fn handler(req, res)`.

use core::marker::PhantomData;
use core::ops::Deref;

use js::{Dict, JsObject, Promise, Unknown};
use node::http::{IncomingMessage, ServerResponse};
use react::ComponentType;

/// [`NextPage`](https://nextjs.org/docs/pages/api-reference/functions/get-initial-props):
/// a page of `Props`, as `NextPage` types it: a component of them.
pub trait NextPage<Props, M>: ComponentType<Props, M> {}

impl<T: ComponentType<Props, M>, Props, M> NextPage<Props, M> for T {}

/// A component Next.js renders, a page, an app or a document, as
/// `NextComponentType` types it: a component of `Props`.
pub trait NextComponentType<Props, M>: ComponentType<Props, M> {}

impl<T: ComponentType<Props, M>, Props, M> NextComponentType<Props, M> for T {}

/// [`NextPageContext`](https://nextjs.org/docs/pages/api-reference/functions/get-initial-props#context-object):
/// what a page's `getInitialProps` is given.
pub struct NextPageContext(PhantomData<JsObject>);

impl NextPageContext {
    /// What rendering it threw, of an error page.
    #[cfg_attr(rust_js, rust_js::link_name = "get err")]
    pub fn err(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    /// Its request, on the server.
    #[cfg_attr(rust_js, rust_js::link_name = "get req")]
    pub fn req(&self) -> Option<&'static IncomingMessage> {
        unreachable!()
    }

    /// Its response, on the server.
    #[cfg_attr(rust_js, rust_js::link_name = "get res")]
    pub fn res(&self) -> Option<&'static ServerResponse> {
        unreachable!()
    }

    /// Its route, `/posts/[id]`.
    #[cfg_attr(rust_js, rust_js::link_name = "get pathname")]
    pub fn pathname(&self) -> String {
        unreachable!()
    }

    /// The URL's query and the route's parameters.
    #[cfg_attr(rust_js, rust_js::link_name = "get query")]
    pub fn query(&self) -> &'static Dict<Unknown> {
        unreachable!()
    }

    /// The path as the browser shows it.
    #[cfg_attr(rust_js, rust_js::link_name = "get asPath")]
    pub fn as_path(&self) -> Option<String> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get locale")]
    pub fn locale(&self) -> Option<String> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get locales")]
    pub fn locales(&self) -> Option<Vec<String>> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get defaultLocale")]
    pub fn default_locale(&self) -> Option<String> {
        unreachable!()
    }

    /// The app's tree, a component a page may render it again of.
    #[cfg_attr(rust_js, rust_js::link_name = "get AppTree")]
    pub fn app_tree(&self) -> &'static react::ComponentValue<Unknown> {
        unreachable!()
    }
}

/// [`NextApiRequest`](https://nextjs.org/docs/pages/building-your-application/routing/api-routes#request-helpers):
/// an API route's request, Node's, its query, cookies and body parsed.
pub struct NextApiRequest(PhantomData<JsObject>);

impl Deref for NextApiRequest {
    type Target = IncomingMessage;

    fn deref(&self) -> &IncomingMessage {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const IncomingMessage) }
    }
}

impl NextApiRequest {
    /// The URL's query, each parameter's value or values.
    #[cfg_attr(rust_js, rust_js::link_name = "get query")]
    pub fn query(&self) -> &'static Dict<QueryValue> {
        unreachable!()
    }

    /// Its cookies, each by its name.
    #[cfg_attr(rust_js, rust_js::link_name = "get cookies")]
    pub fn cookies(&self) -> &'static Dict<String> {
        unreachable!()
    }

    /// Its body, parsed by its `Content-Type`: JSON's, a form's, or its
    /// text.
    #[cfg_attr(rust_js, rust_js::link_name = "get body")]
    pub fn body(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    /// The app's environment variables.
    #[cfg_attr(rust_js, rust_js::link_name = "get env")]
    pub fn env(&self) -> &'static Dict<String> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get draftMode")]
    pub fn draft_mode(&self) -> Option<bool> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get preview")]
    pub fn preview(&self) -> Option<bool> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get previewData")]
    pub fn preview_data(&self) -> Option<&'static Unknown> {
        unreachable!()
    }
}

/// A query parameter's value, or its values, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum QueryValue {
    One(String),
    Many(Vec<String>),
}

/// [`NextApiResponse`](https://nextjs.org/docs/pages/building-your-application/routing/api-routes#response-helpers):
/// an API route's response, Node's, and Next.js's helpers, of a body
/// `Data`.
pub struct NextApiResponse<Data = Unknown>(PhantomData<JsObject>, PhantomData<Data>);

impl<Data> Deref for NextApiResponse<Data> {
    type Target = ServerResponse;

    fn deref(&self) -> &ServerResponse {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const ServerResponse) }
    }
}

impl<Data> NextApiResponse<Data> {
    /// Send `body`, as text, JSON or a buffer by what it is.
    #[cfg_attr(rust_js, rust_js::link_name = "send")]
    pub fn send(&self, body: Data) {
        unreachable!()
    }

    /// Send `body` as JSON.
    #[cfg_attr(rust_js, rust_js::link_name = "json")]
    pub fn json(&self, body: Data) {
        unreachable!()
    }

    /// Set its status, `res.status(200)`, for a `send` or a `json` after.
    #[cfg_attr(rust_js, rust_js::link_name = "status")]
    pub fn status(&self, status_code: u16) -> &Self {
        unreachable!()
    }

    /// Redirect to `url`, a 307.
    #[cfg_attr(rust_js, rust_js::link_name = "redirect")]
    pub fn redirect(&self, url: &str) -> &Self {
        unreachable!()
    }

    /// Redirect to `url` by `status`.
    #[cfg_attr(rust_js, rust_js::link_name = "redirect")]
    pub fn redirect_with_status(&self, status: u16, url: &str) -> &Self {
        unreachable!()
    }

    /// Turn draft mode on or off, by its cookie.
    #[cfg_attr(rust_js, rust_js::link_name = "setDraftMode")]
    pub fn set_draft_mode(&self, options: DraftModeOptions) -> &Self {
        unreachable!()
    }

    /// Turn preview mode on, `data` its data.
    #[cfg_attr(rust_js, rust_js::link_name = "setPreviewData")]
    pub fn set_preview_data(&self, data: &Unknown, options: PreviewDataOptions<'_>) -> &Self {
        unreachable!()
    }

    /// Turn preview mode off.
    #[cfg_attr(rust_js, rust_js::link_name = "clearPreviewData")]
    pub fn clear_preview_data(&self, options: ClearPreviewDataOptions<'_>) -> &Self {
        unreachable!()
    }

    /// Render the static page `url_path` again, on demand.
    #[cfg_attr(rust_js, rust_js::link_name = "revalidate")]
    pub fn revalidate(&self, url_path: &str) -> Promise<()> {
        unreachable!()
    }

    /// [`revalidate`](Self::revalidate), of `options`.
    #[cfg_attr(rust_js, rust_js::link_name = "revalidate")]
    pub fn revalidate_with_options(&self, url_path: &str, options: RevalidateOptions) -> Promise<()> {
        unreachable!()
    }
}

/// [`NextApiResponse::set_draft_mode`]'s.
pub struct DraftModeOptions {
    pub enable: bool,
}

/// [`NextApiResponse::set_preview_data`]'s.
#[derive(Default)]
pub struct PreviewDataOptions<'a> {
    /// How long it lasts, in seconds.
    #[cfg_attr(rust_js, rust_js::name = "maxAge")]
    pub max_age: Option<f64>,
    /// The path it's of.
    pub path: Option<&'a str>,
}

/// [`NextApiResponse::clear_preview_data`]'s.
#[derive(Default)]
pub struct ClearPreviewDataOptions<'a> {
    pub path: Option<&'a str>,
}

/// [`NextApiResponse::revalidate_with_options`]'s.
#[derive(Default)]
pub struct RevalidateOptions {
    /// Only where it's been rendered before.
    pub unstable_only_generated: Option<bool>,
}

/// An API route, as `NextApiHandler` types it: a function of its request
/// and response, `async` or not. `M` only tells it from other traits' impls.
pub trait NextApiHandler<T, M> {}

#[doc(hidden)]
pub struct Handled;

impl<T: 'static, R, F: Fn(&'static NextApiRequest, &'static NextApiResponse<T>) -> R> NextApiHandler<T, Handled> for F {}

/// A page's or an API route's `config`, as `PageConfig` types it.
#[derive(Default)]
pub struct PageConfig<'a> {
    pub api: Option<ApiConfig<'a>>,
    /// The environment variables it reads.
    pub env: Option<&'a [&'a str]>,
    /// How long it may run, in seconds.
    #[cfg_attr(rust_js, rust_js::name = "maxDuration")]
    pub max_duration: Option<f64>,
    /// Its [`ServerRuntime`].
    pub runtime: Option<ServerRuntime<'a>>,
    /// `Some(false)`: the page without its JS.
    #[cfg_attr(rust_js, rust_js::name = "unstable_runtimeJS")]
    pub unstable_runtime_js: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "unstable_JsPreload")]
    pub unstable_js_preload: Option<bool>,
}

/// An API route's limits, in a [`PageConfig`].
#[derive(Default)]
pub struct ApiConfig<'a> {
    /// The most it responds, or `false`, any.
    #[cfg_attr(rust_js, rust_js::name = "responseLimit")]
    pub response_limit: Option<ResponseLimit<'a>>,
    /// How its body is parsed, or `false`, not.
    #[cfg_attr(rust_js, rust_js::name = "bodyParser")]
    pub body_parser: Option<BodyParser<'a>>,
    /// `Some(true)`: it's answered elsewhere, as by Express.
    #[cfg_attr(rust_js, rust_js::name = "externalResolver")]
    pub external_resolver: Option<bool>,
}

/// An [`ApiConfig`]'s `bodyParser`: its size limit, or `false`, not
/// parsed.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum BodyParser<'a> {
    Limit(BodyParserLimit<'a>),
    Bool(bool),
}

/// The most of a body that's parsed.
#[derive(Default)]
pub struct BodyParserLimit<'a> {
    #[cfg_attr(rust_js, rust_js::name = "sizeLimit")]
    pub size_limit: Option<SizeLimit<'a>>,
}

/// Where a page runs, as `ServerRuntime` types it: `"nodejs"`, `"edge"`
/// or `"experimental-edge"`.
pub type ServerRuntime<'a> = &'a str;

/// A size, `number | `${number}${FileSizeSuffix}``: bytes, or a text of
/// them, `"1mb"`, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum SizeLimit<'a> {
    Bytes(f64),
    Str(&'a str),
}

/// A size's suffix, `"kb"`, `"MB"`, `"gb"` and the rest, as
/// `FileSizeSuffix` types it.
pub type FileSizeSuffix<'a> = &'a str;

/// The most a response is, `SizeLimit | boolean`, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ResponseLimit<'a> {
    Bytes(f64),
    Str(&'a str),
    Bool(bool),
}

/// What preview mode holds, as `PreviewData` types it: any value.
pub type PreviewData = Unknown;

/// A route of the app, as `Route` types it: its path.
pub type Route<'a> = &'a str;
