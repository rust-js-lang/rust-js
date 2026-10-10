//! [`url`](https://nodejs.org/api/url.html): URLs. Its WHATWG classes,
//! [`URL`] and [`URLSearchParams`], are the globals, webapi's; its own are
//! file URLs, domains, and a URL written without some of its parts.

use crate::querystring::{ParsedUrlQuery, ParsedUrlQueryInput};

pub use webapi::{
    URL, URLPattern, URLPatternComponentResult, URLPatternInit, URLPatternOptions, URLPatternResult, URLSearchParams,
};

/// What a [`URLSearchParams`]'s `keys`, `values` and `entries` give.
pub type URLSearchParamsIterator<T> = Box<dyn Iterator<Item = T>>;

unsafe extern "Rust" {
    /// [`url.domainToASCII(domain)`](https://nodejs.org/api/url.html#urldomaintoasciidomain):
    /// `domain`'s Punycode, `"xn--espaol-zwa.com"`, or `""` where it isn't one.
    #[link_name = "url#domainToASCII"]
    pub safe fn domain_to_ascii(domain: &str) -> String;

    /// [`url.domainToUnicode(domain)`](https://nodejs.org/api/url.html#urldomaintounicodedomain):
    /// `domain`, from Punycode, `"español.com"`, or `""` where it isn't one.
    #[link_name = "url#domainToUnicode"]
    pub safe fn domain_to_unicode(domain: &str) -> String;

    /// [`url.pathToFileURL(path)`](https://nodejs.org/api/url.html#urlpathtofileurlpath-options):
    /// the `file:` URL of `path`, made absolute, its characters encoded.
    #[link_name = "url#pathToFileURL"]
    pub safe fn path_to_file_url(path: &str) -> &'static URL;

    #[link_name = "url#pathToFileURL"]
    pub safe fn path_to_file_url_with_options(path: &str, options: PathToFileUrlOptions) -> &'static URL;

    /// [`url.resolve(from, to)`](https://nodejs.org/api/url.html#urlresolvefrom-to):
    /// `to`, resolved against `from`, as a browser resolves a link's. Legacy:
    /// `URL`'s base does it.
    #[link_name = "url#resolve"]
    pub safe fn resolve(from: &str, to: &str) -> String;

    /// [`url.format(url)`](https://nodejs.org/api/url.html#urlformaturl-options):
    /// `url`'s text.
    #[link_name = "url#format"]
    pub safe fn format(url: &URL) -> String;

    /// `url.format(url, options)`: `url`'s text, without the parts
    /// `options` leave out.
    #[link_name = "url#format"]
    pub safe fn format_with_options(url: &URL, options: URLFormatOptions) -> String;

    /// `url.format(urlObject)`: the text of a legacy URL's parts. Legacy:
    /// a `URL` of them is the WHATWG one.
    #[link_name = "url#format"]
    pub safe fn format_with_url_object(url_object: UrlObject<'_>) -> String;

    /// `url.format(urlString)`: `urlString`, parsed and written again.
    #[link_name = "url#format"]
    pub safe fn format_with_str(url_string: &str) -> String;

    /// [`url.parse(urlString)`](https://nodejs.org/api/url.html#urlparseurlstring-parsequerystring-slashesdenotehost):
    /// a URL's parts, its query the text. Legacy: `URL` parses as the WHATWG
    /// standard does, where this doesn't.
    #[link_name = "url#parse"]
    pub safe fn parse(url_string: &str) -> UrlWithStringQuery;

    /// `url.parse(urlString, parseQueryString)`: its query parsed, where
    /// `parse_query_string`.
    #[link_name = "url#parse"]
    pub safe fn parse_with_parse_query_string(url_string: &str, parse_query_string: bool) -> Url;

    /// `url.parse(urlString, parseQueryString, slashesDenoteHost)`: `//`
    /// before a host, where `slashes_denote_host`, `//example.com/p`'s.
    #[link_name = "url#parse"]
    pub safe fn parse_with_parse_query_string_and_slashes_denote_host(
        url_string: &str,
        parse_query_string: bool,
        slashes_denote_host: bool,
    ) -> Url;
}

/// What [`parse`] gives: a legacy URL's parts, each `None` where it hasn't
/// one, and its query `Q`: of [`parse_with_parse_query_string`], the text
/// or parsed, [`UrlQuery`].
pub struct Url<Q = Option<UrlQuery>> {
    pub auth: Option<String>,
    pub hash: Option<String>,
    pub host: Option<String>,
    pub hostname: Option<String>,
    pub href: String,
    /// Its pathname and search.
    pub path: Option<String>,
    pub pathname: Option<String>,
    pub protocol: Option<String>,
    pub search: Option<String>,
    pub slashes: Option<bool>,
    pub port: Option<String>,
    pub query: Q,
}

/// A [`Url`] whose query is parsed.
pub type UrlWithParsedQuery = Url<&'static ParsedUrlQuery>;

/// A [`Url`] whose query is its text, [`parse`]'s.
pub type UrlWithStringQuery = Url<Option<String>>;

/// A [`Url`]'s query: its text, or parsed.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum UrlQuery {
    Str(String),
    Parsed(&'static ParsedUrlQuery),
}

/// What [`format_with_url_object`] takes: a legacy URL's parts, each
/// `None` but what's given.
#[derive(Default)]
pub struct UrlObject<'a> {
    pub auth: Option<&'a str>,
    pub hash: Option<&'a str>,
    pub host: Option<&'a str>,
    pub hostname: Option<&'a str>,
    pub href: Option<&'a str>,
    pub pathname: Option<&'a str>,
    pub protocol: Option<&'a str>,
    pub search: Option<&'a str>,
    pub slashes: Option<bool>,
    pub port: Option<UrlObjectPort<'a>>,
    pub query: Option<UrlObjectQuery<'a>>,
}

/// A [`UrlObject`]'s port: a text or a number.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum UrlObjectPort<'a> {
    Str(&'a str),
    Number(f64),
}

/// A [`UrlObject`]'s query: its text, or the values `querystring` writes.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum UrlObjectQuery<'a> {
    Str(&'a str),
    Input(&'a ParsedUrlQueryInput),
}

/// [`url.fileURLToPath(url)`](https://nodejs.org/api/url.html#urlfileurltopathurl-options):
/// the path of a `file:` URL, its characters decoded.
#[cfg_attr(rust_js, rust_js::link_name = "url#fileURLToPath")]
pub fn file_url_to_path(url: impl IntoStrOrURL) -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "url#fileURLToPath")]
pub fn file_url_to_path_with_options(url: impl IntoStrOrURL, options: FileUrlToPathOptions) -> String {
    unreachable!()
}

/// What [`file_url_to_path_with_options`] takes.
#[derive(Default)]
pub struct FileUrlToPathOptions {
    /// Whether the path is Windows', or this platform's where `None`.
    pub windows: Option<bool>,
}

/// What [`path_to_file_url_with_options`] takes.
#[derive(Default)]
pub struct PathToFileUrlOptions {
    /// Whether the path is Windows', or this platform's where `None`.
    pub windows: Option<bool>,
}

/// What [`format_with_options`] takes: whether to write each part, each
/// `true` where `None` but `unicode`.
#[derive(Default)]
pub struct URLFormatOptions {
    /// Its username and password.
    pub auth: Option<bool>,
    /// Its `#` fragment.
    pub fragment: Option<bool>,
    /// Its `?` query.
    pub search: Option<bool>,
    /// Its host's Unicode, where `true`, not Punycode.
    pub unicode: Option<bool>,
}

/// What a `string | URL` parameter takes: each as it is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a `string | URL`")]
#[cfg_attr(rust_js, rust_js::types = "string | URL")]
pub trait IntoStrOrURL: sealed::Sealed {}
impl IntoStrOrURL for &str {}
impl IntoStrOrURL for &String {}
impl IntoStrOrURL for &URL {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &String {}
    impl Sealed for &super::URL {}
}
