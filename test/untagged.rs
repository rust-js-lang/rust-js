//! Untagged enums (ADR 0214) at the JS boundary: what JS passes in is the
//! enum's value, and what Rust gives back is its payload. For the test in
//! compiler.test.ts.

use core::marker::PhantomData;
use core::ops::Deref;
use js::{ArrayBuffer, JsObject, RegExp};

/// JS's `Error`, and its subclass `TypeError`, as `Deref` says.
pub struct Error(PhantomData<JsObject>);
pub struct TypeError(PhantomData<JsObject>);

impl Deref for TypeError {
    type Target = Error;

    fn deref(&self) -> &Error {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const Error) }
    }
}

pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// TS's `string | number | bigint | boolean | string[] | RegExp |
/// ArrayBuffer | Error | TypeError | ((n: number) => number) | Point`.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Src<'a> {
    Text(&'a str),
    Number(f64),
    Big(i64),
    Flag(bool),
    Names(Vec<String>),
    Pattern(&'a RegExp),
    Bytes(&'a ArrayBuffer),
    Failure(&'a Error),
    Mistyped(&'a TypeError),
    Step(&'a dyn Fn(f64) -> f64),
    At(Point),
}

impl<'a> From<&'a str> for Src<'a> {
    fn from(text: &'a str) -> Self {
        Src::Text(text)
    }
}

/// Which variant JS's value is, whatever order the arms are in: a
/// `TypeError` is no plain `Error`, and an array no `Point`.
pub fn kind(src: Src) -> String {
    match src {
        Src::At(p) => format!("point {}", p.x + p.y),
        Src::Failure(_) => "error".to_string(),
        Src::Mistyped(_) => "type error".to_string(),
        Src::Text(t) => format!("text {t}"),
        Src::Number(n) => format!("number {n}"),
        Src::Big(b) => format!("big {b}"),
        Src::Flag(f) => format!("flag {f}"),
        Src::Names(names) => format!("names {}", names.join("+")),
        Src::Pattern(_) => "pattern".to_string(),
        Src::Bytes(b) => format!("bytes {}", js::array_buffer::byte_length(b)),
        Src::Step(f) => format!("step {}", f(1.0)),
    }
}

/// A `&str` into one: the string itself.
pub fn text(s: &str) -> Src<'_> {
    s.into()
}

/// A variant's constructor as a function: the value itself.
pub fn numbers(values: Vec<f64>) -> Vec<Src<'static>> {
    values.into_iter().map(Src::Number).collect()
}

unsafe extern "Rust" {
    /// JS's `String(value)`, given one: the value, as JS takes it.
    #[link_name = "String"]
    safe fn to_text(value: Src<'_>) -> String;
}

pub fn shown(n: f64) -> String {
    to_text(Src::Number(n))
}

/// TS's `string | ReactNode`: text, or whatever else it is, the last
/// variant's, `#[rust_js::otherwise]`, of what the others aren't.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Label<'a> {
    Text(&'a str),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(&'a JsObject),
}

pub fn label(value: Label) -> String {
    let mut label = "Link for this heading".to_string();
    if let Label::Text(text) = value {
        label = format!("Link for {text}");
    }
    label
}

pub fn is_other(value: Label) -> bool {
    matches!(value, Label::Other(_))
}
