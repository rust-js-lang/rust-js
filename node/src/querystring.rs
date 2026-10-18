//! [`querystring`](https://nodejs.org/api/querystring.html): a URL query's
//! text, `a=1&b=2`, from a dictionary of its values, and back. `URL`'s
//! [`URLSearchParams`](crate::url::URLSearchParams) is the WHATWG one.

use js::Dict;

/// [`querystring.stringify(obj, sep, eq, options)`](https://nodejs.org/api/querystring.html#querystringstringifyobj-sep-eq-options):
/// `obj`'s query, `a=1&b=2`, each value encoded; a list's a key each; of
/// `sep` between pairs, or `&`, and `eq` between a key and its value, or
/// `=`.
#[cfg_attr(rust_js, rust_js::link_name = "querystring#stringify")]
pub fn stringify(
    obj: Option<&ParsedUrlQueryInput>,
    sep: Option<&str>,
    eq: Option<&str>,
    options: Option<StringifyOptions>,
) -> String {
    unreachable!()
}

/// [`querystring.encode`](https://nodejs.org/api/querystring.html#querystringencode):
/// [`stringify`], by another name.
#[cfg_attr(rust_js, rust_js::link_name = "querystring#encode")]
pub fn encode(
    obj: Option<&ParsedUrlQueryInput>,
    sep: Option<&str>,
    eq: Option<&str>,
    options: Option<StringifyOptions>,
) -> String {
    unreachable!()
}

/// [`querystring.parse(str, sep, eq, options)`](https://nodejs.org/api/querystring.html#querystringparsestr-sep-eq-options):
/// `str`'s values by their keys, decoded; a key given twice, a list; of
/// `sep` between pairs, or `&`, and `eq` between a key and its value, or
/// `=`.
#[cfg_attr(rust_js, rust_js::link_name = "querystring#parse")]
pub fn parse(str: &str, sep: Option<&str>, eq: Option<&str>, options: Option<ParseOptions>) -> &'static ParsedUrlQuery {
    unreachable!()
}

/// [`querystring.decode`](https://nodejs.org/api/querystring.html#querystringdecode):
/// [`parse`], by another name.
#[cfg_attr(rust_js, rust_js::link_name = "querystring#decode")]
pub fn decode(str: &str, sep: Option<&str>, eq: Option<&str>, options: Option<ParseOptions>) -> &'static ParsedUrlQuery {
    unreachable!()
}

/// [`querystring.escape(str)`](https://nodejs.org/api/querystring.html#querystringescapestr):
/// `str`, percent-encoded as a query's value is.
#[cfg_attr(rust_js, rust_js::link_name = "querystring#escape")]
pub fn escape(str: &str) -> String {
    unreachable!()
}

/// [`querystring.unescape(str)`](https://nodejs.org/api/querystring.html#querystringunescapestr):
/// `str`, its percent-encoding decoded.
#[cfg_attr(rust_js, rust_js::link_name = "querystring#unescape")]
pub fn unescape(str: &str) -> String {
    unreachable!()
}

/// What [`parse`] gives: each key's value.
pub type ParsedUrlQuery = Dict<ParsedUrlQueryValue>;

/// A parsed query's value: one, or those of a key given more than once.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ParsedUrlQueryValue {
    One(String),
    Many(Vec<String>),
}

/// What [`stringify`] takes: each key's value, `None` an empty one.
pub type ParsedUrlQueryInput = Dict<Option<ParsedUrlQueryInputValue>>;

/// A value [`stringify`] writes: a text, a number, a `bool` or a `bigint`,
/// or a list of them, a key each.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ParsedUrlQueryInputValue {
    Str(String),
    Number(f64),
    Bool(bool),
    BigInt(i64),
    List(Vec<ParsedUrlQueryInputItem>),
}

/// An item of a [`ParsedUrlQueryInputValue::List`].
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ParsedUrlQueryInputItem {
    Str(String),
    Number(f64),
    Bool(bool),
    BigInt(i64),
}

/// The options [`stringify`] takes.
#[derive(Default)]
pub struct StringifyOptions {
    /// How a key or a value is encoded, `querystring.escape` where `None`.
    #[cfg_attr(rust_js, rust_js::name = "encodeURIComponent")]
    pub encode_uri_component: Option<Box<dyn Fn(&str) -> String>>,
}

/// The options [`parse`] takes.
#[derive(Default)]
pub struct ParseOptions {
    /// The most keys to read, 1000 where `None`, all where 0.
    #[cfg_attr(rust_js, rust_js::name = "maxKeys")]
    pub max_keys: Option<f64>,
    /// How a key or a value is decoded, `querystring.unescape` where `None`.
    #[cfg_attr(rust_js, rust_js::name = "decodeURIComponent")]
    pub decode_uri_component: Option<Box<dyn Fn(&str) -> String>>,
}
