//! [`http`](https://nodejs.org/api/http.html)'s request and response, as a
//! server's handler, or Next.js's Pages Router, is given them: what's read
//! of a request and written to a response. Their streams' events aren't
//! bound yet.

use core::marker::PhantomData;

use js::{Dict, JsObject};

/// [`http.IncomingMessage`](https://nodejs.org/api/http.html#class-httpincomingmessage):
/// a request a server is given.
pub struct IncomingMessage(PhantomData<JsObject>);

impl IncomingMessage {
    /// Its method, `"GET"`.
    #[cfg_attr(rust_js, rust_js::link_name = "get method")]
    pub fn method(&self) -> Option<String> {
        unreachable!()
    }

    /// Its URL, its path and query, `/search?q=rust`.
    #[cfg_attr(rust_js, rust_js::link_name = "get url")]
    pub fn url(&self) -> Option<String> {
        unreachable!()
    }

    /// Its headers, each by its name in lower case.
    #[cfg_attr(rust_js, rust_js::link_name = "get headers")]
    pub fn headers(&self) -> &'static Dict<IncomingHttpHeader> {
        unreachable!()
    }

    /// Each header's values, by its name in lower case.
    #[cfg_attr(rust_js, rust_js::link_name = "get headersDistinct")]
    pub fn headers_distinct(&self) -> &'static Dict<Vec<String>> {
        unreachable!()
    }

    /// Its headers as they came, each name and then its value.
    #[cfg_attr(rust_js, rust_js::link_name = "get rawHeaders")]
    pub fn raw_headers(&self) -> Vec<String> {
        unreachable!()
    }

    /// Its HTTP version, `"1.1"`.
    #[cfg_attr(rust_js, rust_js::link_name = "get httpVersion")]
    pub fn http_version(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get httpVersionMajor")]
    pub fn http_version_major(&self) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get httpVersionMinor")]
    pub fn http_version_minor(&self) -> f64 {
        unreachable!()
    }

    /// Whether it's been read whole.
    #[cfg_attr(rust_js, rust_js::link_name = "get complete")]
    pub fn complete(&self) -> bool {
        unreachable!()
    }

    /// A response's status, of a client's request.
    #[cfg_attr(rust_js, rust_js::link_name = "get statusCode")]
    pub fn status_code(&self) -> Option<f64> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get statusMessage")]
    pub fn status_message(&self) -> Option<String> {
        unreachable!()
    }

    /// Its trailers, each by its name.
    #[cfg_attr(rust_js, rust_js::link_name = "get trailers")]
    pub fn trailers(&self) -> &'static Dict<String> {
        unreachable!()
    }
}

/// A request's header: one value, or a list of them, `set-cookie`'s.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum IncomingHttpHeader {
    One(String),
    Many(Vec<String>),
}

/// [`http.ServerResponse`](https://nodejs.org/api/http.html#class-httpserverresponse):
/// the response a server writes.
pub struct ServerResponse(PhantomData<JsObject>);

impl ServerResponse {
    /// Its status, `200` unless it's set.
    #[cfg_attr(rust_js, rust_js::link_name = "get statusCode")]
    pub fn status_code(&self) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set statusCode")]
    pub fn set_status_code(&self, value: f64) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get statusMessage")]
    pub fn status_message(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set statusMessage")]
    pub fn set_status_message(&self, value: &str) {
        unreachable!()
    }

    /// Whether its headers are sent.
    #[cfg_attr(rust_js, rust_js::link_name = "get headersSent")]
    pub fn headers_sent(&self) -> bool {
        unreachable!()
    }

    /// Whether it's ended.
    #[cfg_attr(rust_js, rust_js::link_name = "get writableEnded")]
    pub fn writable_ended(&self) -> bool {
        unreachable!()
    }

    /// Set the header `name` to `value`.
    #[cfg_attr(rust_js, rust_js::link_name = "setHeader")]
    pub fn set_header(&self, name: &str, value: impl IntoHeaderValue) -> &Self {
        unreachable!()
    }

    /// Add `value` to the header `name`'s.
    #[cfg_attr(rust_js, rust_js::link_name = "appendHeader")]
    pub fn append_header(&self, name: &str, value: &str) -> &Self {
        unreachable!()
    }

    /// The header `name`'s value, as it's set.
    #[cfg_attr(rust_js, rust_js::link_name = "getHeader")]
    pub fn get_header(&self, name: &str) -> Option<OutgoingHttpHeader> {
        unreachable!()
    }

    /// The names of the headers it's set, in lower case.
    #[cfg_attr(rust_js, rust_js::link_name = "getHeaderNames")]
    pub fn get_header_names(&self) -> Vec<String> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "hasHeader")]
    pub fn has_header(&self, name: &str) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "removeHeader")]
    pub fn remove_header(&self, name: &str) {
        unreachable!()
    }

    /// Send its status, and its headers, now.
    #[cfg_attr(rust_js, rust_js::link_name = "writeHead")]
    pub fn write_head(&self, status_code: f64) -> &Self {
        unreachable!()
    }

    /// Send its headers now.
    #[cfg_attr(rust_js, rust_js::link_name = "flushHeaders")]
    pub fn flush_headers(&self) {
        unreachable!()
    }

    /// Send `chunk` of its body: whether it's sent, not buffered.
    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write(&self, chunk: &str) -> bool {
        unreachable!()
    }

    /// End it.
    #[cfg_attr(rust_js, rust_js::link_name = "end")]
    pub fn end(&self) -> &Self {
        unreachable!()
    }

    /// End it with `chunk`, the last of its body.
    #[cfg_attr(rust_js, rust_js::link_name = "end")]
    pub fn end_with(&self, chunk: &str) -> &Self {
        unreachable!()
    }
}

/// A response's header, as it's set: a number, a text, or a list of texts.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OutgoingHttpHeader {
    Number(f64),
    Str(String),
    Many(Vec<String>),
}

/// What a `number | string | readonly string[]` header value takes: each
/// as it is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a header's value, a number, a `&str` nor a `&[&str]`")]
#[cfg_attr(rust_js, rust_js::types = "number | string | readonly string[]")]
pub trait IntoHeaderValue: sealed::Sealed {}
impl IntoHeaderValue for f64 {}
impl IntoHeaderValue for &str {}
impl IntoHeaderValue for &[&str] {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for f64 {}
    impl Sealed for &str {}
    impl Sealed for &[&str] {}
}
