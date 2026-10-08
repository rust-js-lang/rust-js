//! The JS language for rust-js (ADR 0102): what JS has that Rust's `std`
//! doesn't, as ReScript's standard library has it. Its promises, errors and
//! regular expressions, its byte buffers, its JSON, and its global functions,
//! its timers too, as ReScript's has them: every JS runtime has them. What the
//! browser adds is the webapi crate's; what `std` has, rust-js maps itself.
//!
//! It holds declarations only, so it's never compiled to JS: a program calls
//! what it declares, and the calls become plain JS.

// A JS method of several forms, `slice` and `slice_to_end`, is one link
// name of several Rust signatures, as the webapi crate's are.
#![allow(clashing_extern_declarations)]

use core::marker::PhantomData;

/// `import "./App.css";` in the module's JS, for what a module does when
/// it's loaded, as a bundler's CSS does (ADRs 0039 and 0110): written where
/// it's needed, `js::import!("./App.css");`, as stable Rust has no inner
/// attribute of a tool. It's a `const _` rust-js reads, and writes nothing of.
#[macro_export]
macro_rules! import {
    ($path:literal) => {
        #[cfg_attr(rust_js, rust_js::import = $path)]
        const _: () = ();
    };
}

/// What the module runs when it's loaded, its JS's own statements, as a
/// prefetch of what it imports later: `js::on_load! { prefetch(); }` (ADR
/// 0267). rustc checks them as a function's body, which nothing calls.
#[macro_export]
macro_rules! on_load {
    ($($body:tt)*) => {
        const _: () = {
            #[cfg_attr(rust_js, rust_js::on_load)]
            #[allow(dead_code)]
            fn on_load() {
                $($body)*
            }
        };
    };
}

/// The module's directive, the first statement of its JS: `"use client";`
/// of a React component a Next.js Server Component renders, written
/// `js::directive!("use client");` (ADR 0192).
#[macro_export]
macro_rules! directive {
    ($directive:literal) => {
        #[cfg_attr(rust_js, rust_js::directive = $directive)]
        const _: () = ();
    };
}

/// The module's default export, a function of its own, which keeps its
/// name too: `js::export_default!(page);` is `export default page;`, what a
/// Next.js route's `page.jsx` has (ADR 0192). The function is named in a
/// `const _`'s `use`, so a plain rustc checks it's there, a generic one's
/// too.
#[macro_export]
macro_rules! export_default {
    ($function:path) => {
        #[cfg_attr(rust_js, rust_js::export_default)]
        const _: () = {
            #[allow(unused_imports)]
            use $function as _;
        };
    };
}

/// The crate's own functions, fields and props are camelCase in JS, as its
/// variables are (ADRs 0046 and 0110): `js::camel_case!();` at the crate root.
#[macro_export]
macro_rules! camel_case {
    () => {
        #[cfg_attr(rust_js, rust_js::camel_case)]
        const _: () = ();
    };
}

/// Any JS object. Every type here and in webapi holds a `PhantomData` of
/// it, which is how rust-js knows it's a JS object, and so may a program's
/// own: `pub struct EditorView(PhantomData<JsObject>);` (ADR 0111). A program
/// never has one by value, only a reference a binding gives: its field is
/// private, so nothing makes one, and it's neither `Send` nor `Sync`.
#[cfg_attr(rust_js, rust_js::js_object)]
pub struct JsObject(PhantomData<*mut ()>);

/// A JS value of unknown shape (ADR 0225), as TypeScript's `unknown` and
/// ReScript's are: what `JSON.parse` gives, or a binding's of `any`. Never
/// `undefined` nor `null`, which an `Option<&Unknown>` is `None` of (ADR
/// 0030). [`classify`] tells what it is, and [`get`] and [`set`] reach a
/// property of it by name.
#[cfg_attr(rust_js, rust_js::types = "unknown")]
pub struct Unknown(PhantomData<JsObject>);

/// What an [`Unknown`] is, as JS's `typeof` and `Array.isArray` tell it:
/// each variant's value is the value itself (ADR 0214). An array's items
/// may be `undefined` or `null`; a function is a `Function`; anything else,
/// an object or a symbol, is an `Object`, whose properties [`get`] reads.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Kind<'a> {
    String(&'a str),
    Number(f64),
    /// A `bigint`, as an `i64` is one (ADR 0086): one wider isn't wrapped.
    BigInt(i64),
    Bool(bool),
    /// A function, `typeof value === "function"`, of whatever it takes.
    Function(&'a dyn Fn()),
    Array(&'a [Option<&'a Unknown>]),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Object(&'a Unknown),
}

/// What `value` is: `match classify(value) { Kind::String(s) => .., .. }`,
/// a `typeof` of it in JS. The value itself.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn classify(this: &Unknown) -> Kind<'_> {
    unreachable!()
}

/// `value` as a JS value of any shape, which it is: what's never `undefined`
/// nor `null` (`Defined`), text or a number, say, as any value is
/// TypeScript's `unknown`. The value itself.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn unknown<T: Defined + ?Sized>(this: &T) -> &Unknown {
    unreachable!()
}

/// [`String(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/String):
/// any value as text, as JS makes it, `result += value` say: `"undefined"`
/// of `None`, `"[object Object]"` of an object.
#[cfg_attr(rust_js, rust_js::link_name = "String")]
#[allow(unused_variables)]
pub fn string(value: Option<&Unknown>) -> String {
    unreachable!()
}

/// `value[key]`: a property of an object, by its name; `None` where it's
/// `undefined` or `null`, as where there's none.
#[cfg_attr(rust_js, rust_js::link_name = "get []")]
#[allow(unused_variables)]
pub fn get(this: &Unknown, key: &str) -> Option<&'static Unknown> {
    unreachable!()
}

/// `value[key] = to`: `to` as JS has it, a string or a number, or an
/// object.
#[cfg_attr(rust_js, rust_js::link_name = "set []")]
#[allow(unused_variables)]
pub fn set<T>(this: &Unknown, key: &str, to: T) {
    unreachable!()
}

/// A value the browser's [structured clone](https://developer.mozilla.org/docs/Web/API/Web_Workers_API/Structured_clone_algorithm)
/// copies as it is (ADR 0225): what `postMessage`, `pushState` and
/// `structuredClone` take, which a closure or a DOM node isn't, nor
/// `Some(None)`, whose box would arrive as an object. Numbers, strings and
/// `bool`s are, and arrays, tuples and `Vec`s of them, an `Option` of a
/// [`Defined`] one, `Json`, `Dict`, `Unknown`, and what WebIDL marks
/// `[Serializable]`, a `Blob`. A struct is one as its fields are: unsafe to
/// implement, as its `impl` vouches they are, `unsafe impl StructuredClone for Saved {}`.
pub unsafe trait StructuredClone {}

/// A value that's never `undefined` nor `null` in JS, as `None` is: an
/// `Option` of one is the value or `undefined`, not a box (ADR 0051), so
/// one a clone copies as it is.
pub unsafe trait Defined {}

/// Numbers, strings and the js crate's own: each copied, and never nullish.
macro_rules! cloned {
    ($($t:ty),* $(,)?) => {
        $(
            unsafe impl StructuredClone for $t {}
            unsafe impl Defined for $t {}
        )*
    };
}

cloned!(bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, char, str, String);
cloned!(Unknown, ArrayBuffer, Uint8Array, Json<'_>);

unsafe impl<T: StructuredClone + ?Sized> StructuredClone for &T {}
unsafe impl<T: Defined + ?Sized> Defined for &T {}
unsafe impl<T: StructuredClone + ?Sized> StructuredClone for Box<T> {}
unsafe impl<T: Defined + ?Sized> Defined for Box<T> {}
unsafe impl<T: StructuredClone> StructuredClone for [T] {}
unsafe impl<T> Defined for [T] {}
unsafe impl<T: StructuredClone, const N: usize> StructuredClone for [T; N] {}
unsafe impl<T, const N: usize> Defined for [T; N] {}
unsafe impl<T: StructuredClone> StructuredClone for Vec<T> {}
unsafe impl<T> Defined for Vec<T> {}
unsafe impl<T: StructuredClone> StructuredClone for Dict<T> {}
unsafe impl<T> Defined for Dict<T> {}
// `Some(x)` is `x` itself only where `x` can't look like `None`.
unsafe impl<T: StructuredClone + Defined> StructuredClone for Option<T> {}

/// A tuple is an array (ADR 0020): copied if its items are, never nullish.
macro_rules! tuples {
    ($(($($t:ident),+))*) => {
        $(
            unsafe impl<$($t: StructuredClone),+> StructuredClone for ($($t,)+) {}
            unsafe impl<$($t),+> Defined for ($($t,)+) {}
        )*
    };
}

tuples! {
    (A)
    (A, B)
    (A, B, C)
    (A, B, C, D)
    (A, B, C, D, E)
    (A, B, C, D, E, F)
}

/// A plain JS object of `T`s by their names (ADR 0225), as ReScript's `dict`
/// and TypeScript's `Record<string, T>` are: a JSON object, or a
/// dictionary an API takes. [`dict::get`] reads one by its key.
#[cfg_attr(rust_js, rust_js::types = "{ [key: string]: T }")]
pub struct Dict<T>(PhantomData<JsObject>, PhantomData<T>);

/// A JSON value (ADR 0225), as ReScript's `JSON.t` is: each variant's value
/// is the value itself (ADR 0214), and a JSON `null` is `None` of the
/// `Option<Json>` that holds it, an array's item or an object's.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy)]
pub enum Json<'a> {
    String(&'a str),
    Number(f64),
    Bool(bool),
    Array(&'a [Option<Json<'a>>]),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Object(&'a Dict<Option<Json<'a>>>),
}

impl Json<'static> {
    /// [`JSON.parse(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/parse):
    /// the value `text` is, `None` of `null`, or what it threw for text that
    /// isn't JSON (ADR 0035).
    #[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
    #[allow(unused_variables)]
    pub fn parse(text: &str) -> Result<Option<Json<'static>>, &'static JsError> {
        unreachable!()
    }
}

impl Json<'_> {
    /// [`JSON.stringify(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/stringify):
    /// its JSON text.
    #[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
    #[allow(unused_variables)]
    pub fn stringify(value: &Json<'_>) -> String {
        unreachable!()
    }
}

/// A [`Dict`]'s functions.
pub mod dict {
    use super::*;

    /// Its value of `key`: `None` where the key isn't its own, and of a
    /// `Dict<Option<_>>`, `Some(None)` of a JSON `null`.
    #[cfg_attr(rust_js, rust_js::link_name = "@rust-js/runtime#$dictGet")]
    #[allow(unused_variables)]
    pub fn get<'a, T>(dict: &'a Dict<T>, key: &str) -> Option<&'a T> {
        unreachable!()
    }

    /// `dict[key] = value`.
    #[cfg_attr(rust_js, rust_js::link_name = "set []")]
    #[allow(unused_variables)]
    pub fn set<T>(this: &Dict<T>, key: &str, value: T) {
        unreachable!()
    }

    /// [`Object.keys(dict)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/keys):
    /// its keys, in order.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.keys")]
    #[allow(unused_variables)]
    pub fn keys<T>(dict: &Dict<T>) -> Vec<String> {
        unreachable!()
    }

    /// [`Object.entries(dict)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/entries):
    /// each key and its value, in order.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.entries")]
    #[allow(unused_variables)]
    pub fn entries<T>(dict: &Dict<T>) -> Vec<(String, T)> {
        unreachable!()
    }

    /// [`Object.fromEntries(entries)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/fromEntries):
    /// a dictionary of these keys and values.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.fromEntries")]
    #[allow(unused_variables)]
    pub fn from_entries<T>(entries: Vec<(String, T)>) -> &'static Dict<T> {
        unreachable!()
    }
}

/// A JS [`Promise`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise)
/// of a `T`. `.await` on one is JS's `await`; a rejected one throws, like a
/// panic. See ADR 0029.
pub struct Promise<T>(PhantomData<JsObject>, PhantomData<T>);

impl<T> Promise<T> {
    /// [`promise.then(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/then):
    /// a promise of what `f` makes of what it fulfils, as ReScript's
    /// `Promise.thenResolve`. `f` gives no promise, which JS would wait for.
    #[cfg_attr(rust_js, rust_js::link_name = "then")]
    #[allow(unused_variables)]
    pub fn then_resolve<U>(self, f: impl FnOnce(T) -> U + 'static) -> Promise<U> {
        unreachable!()
    }
}

impl<T> core::future::Future for Promise<T> {
    type Output = T;

    fn poll(self: core::pin::Pin<&mut Self>, _: &mut core::task::Context<'_>) -> core::task::Poll<T> {
        unreachable!("rust-js compiles `.await` to JS's `await`")
    }
}

/// `promise`, settled either way: its `.await` is `Ok` of what it fulfils
/// with, or `Err` of what it's rejected with, where the `.await` of the
/// promise itself would throw. For a promise of the webapi crate's, as
/// `settle(window::fetch(window, url)).await` is a network error's `Err`
/// (ADR 0035).
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn settle<T>(this: Promise<T>) -> Promise<Result<T, &'static JsError>> {
    unreachable!()
}

/// A future as the promise it is in JS, where a call of an `async fn` is
/// one already: handed, unawaited, to what takes a promise, as
/// `new ClipboardItem({ "text/plain": promise(fetch_blob()) })`.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn promise<T>(this: impl core::future::Future<Output = T>) -> Promise<T> {
    unreachable!()
}

unsafe extern "Rust" {
    /// Run a future without waiting for it, as from an event handler:
    /// `spawn(Box::new(async move { .. }))`. A JS promise is already
    /// running, so in JS this is the promise itself, left unawaited.
    #[link_name = "this"]
    pub safe fn spawn(this: Box<dyn core::future::Future<Output = ()>>);

    /// [`encodeURIComponent`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/encodeURIComponent):
    /// `text` for a part of a URL, a query's value say.
    #[link_name = "encodeURIComponent"]
    pub safe fn encode_uri_component(text: &str) -> String;

    /// [`encodeURI`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/encodeURI):
    /// `text` for a whole URL, its `/`, `?` and `#` kept.
    #[link_name = "encodeURI"]
    pub safe fn encode_uri(text: &str) -> String;

    /// [`decodeURIComponent`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/decodeURIComponent):
    /// what `encode_uri_component` made, or the `URIError` of what it can't have.
    #[link_name = "decodeURIComponent"]
    pub safe fn decode_uri_component(text: &str) -> Result<String, &'static JsError>;

    /// [`decodeURI`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/decodeURI):
    /// what `encode_uri` made, or the `URIError` of what it can't have.
    #[link_name = "decodeURI"]
    pub safe fn decode_uri(text: &str) -> Result<String, &'static JsError>;
}

/// What `set_timeout` gives, to clear it with: a number in a browser, an
/// object in Node.
pub struct TimeoutId(PhantomData<JsObject>);

/// What `set_interval` gives, to clear it with.
pub struct IntervalId(PhantomData<JsObject>);

// Timers: not the language's own, but every JS runtime's, browsers', workers'
// and Node's, as ReScript's standard library has them (ADR 0102).
unsafe extern "Rust" {
    /// [`setTimeout(callback, ms)`](https://developer.mozilla.org/docs/Web/API/Window/setTimeout):
    /// `callback` once, after at least `ms` milliseconds.
    #[link_name = "setTimeout"]
    pub safe fn set_timeout(callback: Box<dyn FnOnce()>, ms: u32) -> &'static TimeoutId;

    /// [`clearTimeout(id)`](https://developer.mozilla.org/docs/Web/API/Window/clearTimeout):
    /// the timeout's `callback` won't run, if it hasn't.
    #[link_name = "clearTimeout"]
    pub safe fn clear_timeout(id: &TimeoutId);

    /// [`setInterval(callback, ms)`](https://developer.mozilla.org/docs/Web/API/Window/setInterval):
    /// `callback` every `ms` milliseconds, until it's cleared.
    #[link_name = "setInterval"]
    pub safe fn set_interval(callback: Box<dyn FnMut()>, ms: u32) -> &'static IntervalId;

    /// [`clearInterval(id)`](https://developer.mozilla.org/docs/Web/API/Window/clearInterval):
    /// the interval's `callback` doesn't run again.
    #[link_name = "clearInterval"]
    pub safe fn clear_interval(id: &IntervalId);
}

/// A JS [`RegExp`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp),
/// for what Rust would use the `regex` crate for. `replace` with a string is
/// here; with a closure, and `matchAll`, are bindings a program declares, typed
/// for its pattern: a JS replacer is given the match, then each group, then
/// where it matched, so its Rust type is the pattern's.
pub struct RegExp(PhantomData<JsObject>);

pub mod reg_exp {
    use super::*;

    unsafe extern "Rust" {
        /// `new RegExp(pattern, flags)`: flags like `"gm"`.
        /// Of a pattern and flags written as they are, it's their literal,
        /// `/%s/g`, as JS writes one.
        #[link_name = "new RegExp"]
        pub safe fn new(pattern: &str, flags: &str) -> &'static RegExp;

        #[link_name = "test"]
        pub safe fn test(this: &RegExp, text: &str) -> bool;

        /// [`text.replace(pattern, with)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/replace):
        /// the first match, or with the `g` flag each, replaced with `with`,
        /// in which `$1` is the first group and `$&` the match.
        #[link_name = "replace"]
        pub safe fn replace(this: &str, pattern: &RegExp, with: &str) -> String;
    }
}

/// A JS string's methods, as ReScript's standard library names them, for
/// code that counts as JS does: by UTF-16 code units, where Rust's `str`
/// counts bytes, so `"😀".len()` is 4 and its JS `length` 2. A negative
/// index of `slice` counts from the end.
pub mod string {
    unsafe extern "Rust" {
        /// [`text.length`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/length):
        /// its UTF-16 code units.
        #[link_name = "get length"]
        pub safe fn length(this: &str) -> u32;

        /// [`text.charAt(index)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/charAt):
        /// its UTF-16 code unit at `index`, half of an emoji, or `""` past its end.
        #[link_name = "charAt"]
        pub safe fn char_at(this: &str, index: u32) -> String;

        /// [`text.slice(start, end)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/slice):
        /// from `start` to before `end`, each counted from the end if negative.
        #[link_name = "slice"]
        pub safe fn slice(this: &str, start: i32, end: i32) -> String;

        /// `text.slice(start)`: from `start` to the end.
        #[link_name = "slice"]
        pub safe fn slice_to_end(this: &str, start: i32) -> String;

        /// [`text.substring(start, end)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/substring):
        /// from `start` to before `end`, the two swapped if `start` is after it.
        #[link_name = "substring"]
        pub safe fn substring(this: &str, start: u32, end: u32) -> String;

        /// `text.substring(start)`: from `start` to the end.
        #[link_name = "substring"]
        pub safe fn substring_to_end(this: &str, start: u32) -> String;

        /// [`text.indexOf(search)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/indexOf):
        /// where `search` first is, or `-1`.
        #[link_name = "indexOf"]
        pub safe fn index_of(this: &str, search: &str) -> i32;

        /// `text.indexOf(search, from)`: where `search` first is from `from` on, or `-1`.
        #[link_name = "indexOf"]
        pub safe fn index_of_from(this: &str, search: &str, from: u32) -> i32;

        /// [`text.lastIndexOf(search)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/lastIndexOf):
        /// where `search` last is, or `-1`.
        #[link_name = "lastIndexOf"]
        pub safe fn last_index_of(this: &str, search: &str) -> i32;

        /// [`text.replace(pattern, replacement)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/replace):
        /// its first `pattern` replaced, where Rust's `str::replace` replaces each;
        /// `        /// [`text.trim()`]` in `replacement` is the match.
        #[link_name = "replace"]
        pub safe fn replace(this: &str, pattern: &str, replacement: &str) -> String;

        /// [`text.trim()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/trim):
        /// without JS's white space and line ends at either end, which aren't
        /// Rust's `trim`'s quite: JS's has U+FEFF, Rust's U+0085.
        #[link_name = "trim"]
        pub safe fn trim(this: &str) -> String;

        /// `text.trimStart()`: without JS's white space at its start.
        #[link_name = "trimStart"]
        pub safe fn trim_start(this: &str) -> String;

        /// `text.trimEnd()`: without JS's white space at its end.
        #[link_name = "trimEnd"]
        pub safe fn trim_end(this: &str) -> String;
    }
}

/// A JS number's methods, for where Rust's own `format!` isn't what's wanted.
pub mod number {
    unsafe extern "Rust" {
        /// [`x.toFixed(digits)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Number/toFixed):
        /// `x` with `digits` decimals, as JS rounds, a tie away from zero:
        /// `2.5` is `"3"`, where `format!("{:.0}", 2.5)` is `"2"`, to even, as
        /// Rust rounds it. Between ties they agree. `-0` is `"0"`, `1e21` and up
        /// is `String(x)`, `"1e+21"`, and `Infinity` is `"Infinity"`, where
        /// `format!`'s is `"inf"`. `digits` is 0 to 100: more throws a `RangeError`.
        #[link_name = "toFixed"]
        pub safe fn to_fixed(this: f64, digits: u32) -> String;
    }
}

/// JS's [`JSON`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON).
/// A Rust value's JSON is serde's (ADR 0077): `JSON.stringify` of one, as
/// rust-js has it in JS, isn't, since `None` is `undefined` and an `i64` a
/// `BigInt`, which it throws on. So it's typed for what it's exact for.
pub mod json {
    use super::*;

    unsafe extern "Rust" {
        /// [`JSON.parse(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/parse):
        /// the value `text` is, `None` of `null`, or what it threw for JSON
        /// that isn't (ADR 0035).
        #[link_name = "JSON.parse"]
        pub safe fn parse(text: &str) -> Result<Option<&'static Unknown>, &'static JsError>;

        /// [`JSON.stringify(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/stringify):
        /// `text`'s JSON, in quotes, with its `"`, `\` and control characters
        /// escaped, which is a JS string literal too.
        #[link_name = "JSON.stringify"]
        pub safe fn stringify(text: &str) -> String;
    }
}

/// Whatever a JS function threw, or a promise rejected with: usually an
/// [`Error`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error).
/// An `extern` function that returns `Result<T, &JsError>` catches it (ADR 0035).
pub struct JsError(PhantomData<JsObject>);

pub mod js_error {
    use super::*;

    unsafe extern "Rust" {
        /// `String(e)`: an `Error`'s name and message, or any value as text.
        #[link_name = "String"]
        pub safe fn to_string(error: &JsError) -> String;

        /// `e instanceof Error`: what was thrown is an
        /// [`Error`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error),
        /// which has a `message`. A promise may be rejected with anything.
        #[link_name = "instanceof Error"]
        pub safe fn is_error(this: &JsError) -> bool;

        /// [`e.message`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error/message):
        /// an `Error`'s message, without its name, as `is_error(e)` says it is one.
        #[link_name = "get message"]
        pub safe fn message(this: &JsError) -> String;
    }
}

/// JS's [`Object`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object)
/// functions, for a JS object an API takes or gives.
pub mod object {
    use super::*;

    unsafe extern "Rust" {
        /// [`Object.keys(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/keys):
        /// the names of an object's own properties, in order, each one
        /// [`get`] reads.
        #[link_name = "Object.keys"]
        pub safe fn keys(value: &Unknown) -> Vec<String>;
    }

    /// [`Object.fromEntries(entries)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/fromEntries):
    /// a JS object of these keys and values, as an API that takes a
    /// dictionary of them wants: `{ "aria-label": "Rust source" }`.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.fromEntries")]
    #[allow(unused_variables)]
    pub fn from_entries<T>(entries: Vec<(String, T)>) -> &'static JsObject {
        unreachable!()
    }

    /// [`Object.is(a, b)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/is):
    /// the same object, or the same value, where `NaN` is itself and `0`
    /// isn't `-0`. Of JS objects, whether they're one: `std::ptr::eq` isn't
    /// rust-js's. Of two Rust types too, since a JS object may be seen as
    /// either: a message's sender, an object, and a frame's `Window`.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.is")]
    #[allow(unused_variables)]
    pub fn is<A: ?Sized, B: ?Sized>(a: &A, b: &B) -> bool {
        unreachable!()
    }
}

/// A JS [`ArrayBuffer`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/ArrayBuffer):
/// raw bytes, as `response::array_buffer` gives them.
pub struct ArrayBuffer(PhantomData<JsObject>);

pub mod array_buffer {
    use super::*;

    unsafe extern "Rust" {
        #[link_name = "get byteLength"]
        pub safe fn byte_length(this: &ArrayBuffer) -> u32;
    }
}

/// A JS [`Uint8Array`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array):
/// a view of the bytes in an `ArrayBuffer`, as `response::bytes` gives them.
pub struct Uint8Array(PhantomData<JsObject>);

pub mod uint8_array {
    use super::*;

    unsafe extern "Rust" {
        /// A view of all of `buffer`.
        #[link_name = "new Uint8Array"]
        pub safe fn new(buffer: &ArrayBuffer) -> &'static Uint8Array;

        /// How many bytes it views.
        #[link_name = "get length"]
        pub safe fn length(this: &Uint8Array) -> u32;

        /// The buffer it views.
        #[link_name = "get buffer"]
        pub safe fn buffer(this: &Uint8Array) -> &'static ArrayBuffer;
    }
}
