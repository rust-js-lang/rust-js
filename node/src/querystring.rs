//! [`querystring`](https://nodejs.org/api/querystring.html): a URL query's
//! text, `a=1&b=2`, from a dictionary of its values, and back. `URL`'s
//! [`URLSearchParams`](crate::url::URLSearchParams) is the WHATWG one.

use js::Dict;

unsafe extern "Rust" {
    /// [`querystring.stringify(obj)`](https://nodejs.org/api/querystring.html#querystringstringifyobj-sep-eq-options):
    /// `obj`'s query, `a=1&b=2`, each value encoded; a list's a key each.
    #[link_name = "querystring#stringify"]
    pub safe fn stringify(obj: &ParsedUrlQueryInput) -> String;

    /// `querystring.stringify(obj, sep)`: of `sep` between pairs.
    #[link_name = "querystring#stringify"]
    pub safe fn stringify_with_sep(obj: &ParsedUrlQueryInput, sep: &str) -> String;

    /// `querystring.stringify(obj, sep, eq)`: of `eq` between a key and its value.
    #[link_name = "querystring#stringify"]
    pub safe fn stringify_with_sep_and_eq(obj: &ParsedUrlQueryInput, sep: &str, eq: &str) -> String;

    #[link_name = "querystring#stringify"]
    pub safe fn stringify_with_sep_and_eq_and_options(
        obj: &ParsedUrlQueryInput,
        sep: &str,
        eq: &str,
        options: StringifyOptions,
    ) -> String;

    /// [`querystring.encode`](https://nodejs.org/api/querystring.html#querystringencode):
    /// [`stringify`], by another name.
    #[link_name = "querystring#encode"]
    pub safe fn encode(obj: &ParsedUrlQueryInput) -> String;

    #[link_name = "querystring#encode"]
    pub safe fn encode_with_sep(obj: &ParsedUrlQueryInput, sep: &str) -> String;

    #[link_name = "querystring#encode"]
    pub safe fn encode_with_sep_and_eq(obj: &ParsedUrlQueryInput, sep: &str, eq: &str) -> String;

    #[link_name = "querystring#encode"]
    pub safe fn encode_with_sep_and_eq_and_options(
        obj: &ParsedUrlQueryInput,
        sep: &str,
        eq: &str,
        options: StringifyOptions,
    ) -> String;

    /// [`querystring.parse(str)`](https://nodejs.org/api/querystring.html#querystringparsestr-sep-eq-options):
    /// `str`'s values by their keys, decoded; a key given twice, a list.
    #[link_name = "querystring#parse"]
    pub safe fn parse(str: &str) -> &'static ParsedUrlQuery;

    #[link_name = "querystring#parse"]
    pub safe fn parse_with_sep(str: &str, sep: &str) -> &'static ParsedUrlQuery;

    #[link_name = "querystring#parse"]
    pub safe fn parse_with_sep_and_eq(str: &str, sep: &str, eq: &str) -> &'static ParsedUrlQuery;

    #[link_name = "querystring#parse"]
    pub safe fn parse_with_sep_and_eq_and_options(str: &str, sep: &str, eq: &str, options: ParseOptions) -> &'static ParsedUrlQuery;

    /// [`querystring.decode`](https://nodejs.org/api/querystring.html#querystringdecode):
    /// [`parse`], by another name.
    #[link_name = "querystring#decode"]
    pub safe fn decode(str: &str) -> &'static ParsedUrlQuery;

    #[link_name = "querystring#decode"]
    pub safe fn decode_with_sep(str: &str, sep: &str) -> &'static ParsedUrlQuery;

    #[link_name = "querystring#decode"]
    pub safe fn decode_with_sep_and_eq(str: &str, sep: &str, eq: &str) -> &'static ParsedUrlQuery;

    #[link_name = "querystring#decode"]
    pub safe fn decode_with_sep_and_eq_and_options(str: &str, sep: &str, eq: &str, options: ParseOptions) -> &'static ParsedUrlQuery;

    /// [`querystring.escape(str)`](https://nodejs.org/api/querystring.html#querystringescapestr):
    /// `str`, percent-encoded as a query's value is.
    #[link_name = "querystring#escape"]
    pub safe fn escape(str: &str) -> String;

    /// [`querystring.unescape(str)`](https://nodejs.org/api/querystring.html#querystringunescapestr):
    /// `str`, its percent-encoding decoded.
    #[link_name = "querystring#unescape"]
    pub safe fn unescape(str: &str) -> String;
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

/// What [`stringify_with_sep_and_eq_and_options`] takes.
#[derive(Default)]
pub struct StringifyOptions {
    /// How a key or a value is encoded, `querystring.escape` where `None`.
    #[cfg_attr(rust_js, rust_js::name = "encodeURIComponent")]
    pub encode_uri_component: Option<Box<dyn Fn(&str) -> String>>,
}

/// What [`parse_with_sep_and_eq_and_options`] takes.
#[derive(Default)]
pub struct ParseOptions {
    /// The most keys to read, 1000 where `None`, all where 0.
    #[cfg_attr(rust_js, rust_js::name = "maxKeys")]
    pub max_keys: Option<f64>,
    /// How a key or a value is decoded, `querystring.unescape` where `None`.
    #[cfg_attr(rust_js, rust_js::name = "decodeURIComponent")]
    pub decode_uri_component: Option<Box<dyn Fn(&str) -> String>>,
}
