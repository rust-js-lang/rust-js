//! The builtins crate (ADR 0102) as a program calls it: each function is the
//! JS global or method it names. For the test in compiler.test.ts.

use js::{Unknown, json, reg_exp, string};

/// JSON's value, each property a reviver gives none of left out, as
/// react.dev's errors page revives its elements.
pub fn revived(text: &str) -> Option<&'static Unknown> {
    json::parse_with(text, Box::new(hidden))
}

/// Whether `value` has a property of the name, its own or its prototype's,
/// as react.dev's errors page asks of its MDX components.
pub fn holds(value: &Unknown, key: &str) -> bool {
    js::has(value, key)
}

/// A property by a name that's itself a JS value, as react.dev's errors page
/// looks up the component a JSON element names.
pub fn named(value: &Unknown, key: &Unknown) -> Option<&'static Unknown> {
    if js::has(value, key) { js::get(value, key) } else { None }
}

/// Whether `value` has a property of the name a key that's none has,
/// `"undefined"`, as react.dev's errors page asks of a type it may not have.
pub fn holds_none(value: &Unknown, key: Option<&Unknown>) -> bool {
    js::has(value, &key)
}

pub struct Wrapper {
    pub children: Option<&'static Unknown>,
}

// An object, never `undefined`.
unsafe impl js::Defined for Wrapper {}

/// A struct given by value as a JS value of any shape, an element's props as
/// react.dev's errors page makes them.
pub fn wrapped(children: Option<&'static Unknown>) -> &'static Unknown {
    js::unknown_of(Wrapper { children })
}

/// Text that may be none, as a JS value of any shape: the option itself,
/// as react.dev's createFileMap shows a code block's children.
pub fn as_unknown(text: Option<&str>) -> Option<&Unknown> {
    text.map(js::unknown)
}

/// A dictionary made empty, then filled, as react.dev's createFileMap makes
/// Sandpack's files.
pub fn filled() -> &'static js::Dict<u32> {
    let files = js::dict::new();
    js::dict::set(files, "/App.js", 1);
    files
}

fn hidden(key: &str, value: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if key == "secret" { None } else { value }
}

/// A string's JSON text, which is a JS string literal too.
pub fn quoted(text: &str) -> String {
    json::stringify(text)
}

/// `pattern`'s matches replaced with `with`, in which `$1` is the first group.
pub fn replaced(text: &str, pattern: &str, flags: &str, with: &str) -> String {
    reg_exp::replace(text, reg_exp::new(pattern, flags), with)
}

/// JS's string methods, by JS's indexes, UTF-16's, as react.dev's code
/// reads its strings: each part of `text`, and where `part` is in it.
pub fn parts(text: &str, part: &str) -> (Vec<String>, Vec<i32>, u32) {
    let parts = vec![
        string::slice(text, 1, -1),
        string::slice_to_end(text, -2),
        string::substring(text, 1, 3),
        string::substring_to_end(text, 1),
        string::trim(text),
        string::trim_start(text),
        string::trim_end(text),
    ];
    let at = vec![
        string::index_of(text, part),
        string::index_of_from(text, part, 2),
        string::last_index_of(text, part),
    ];
    (parts, at, string::length(text))
}

/// JS's `replace` of a string: the first match only, where Rust's
/// `str::replace` replaces each.
pub fn replaced_first(text: &str, from: &str, to: &str) -> String {
    string::replace(text, from, to)
}

/// JS's `split` of a string, by text and by a pattern of no groups, as
/// react.dev's pages take a path's query and hash off, `/[\?\#]/`.
pub fn split_parts(text: &str) -> (Vec<String>, Vec<String>) {
    (string::split(text, ","), string::split_by_reg_exp(text, reg_exp::new(r"[,;]", "")))
}

/// Each UTF-16 code unit of `text`, by JS's `charAt`: an emoji's two halves.
pub fn units(text: &str) -> Vec<String> {
    (0..string::length(text)).map(|i| string::char_at(text, i)).collect()
}

/// A JS object of these keys and values, as an API taking a dictionary wants.
pub fn attributes(label: &str, level: u32) -> &'static js::JsObject {
    js::object::from_entries(vec![("aria-label".to_string(), label.to_string()), ("aria-level".to_string(), level.to_string())])
}

/// `Object.is`: the same object, or the same value, NaN too.
pub fn identical(same: bool) -> (bool, bool) {
    let a = js::object::from_entries(vec![("k".to_string(), 1)]);
    let b = if same { a } else { js::object::from_entries(vec![("k".to_string(), 1)]) };
    (js::object::is(a, b), js::object::is(&f64::NAN, &f64::NAN))
}

unsafe extern "Rust" {
    #[link_name = "Promise.reject"]
    safe fn rejected(reason: &str) -> js::Promise<Result<u32, &'static js::JsError>>;
}

/// What was thrown: an `Error`'s message, or that it's no `Error`.
pub async fn thrown() -> (String, String) {
    let error = js::decode_uri_component("%E0%A4%A").unwrap_err();
    let first = if error.is_error() { error.message() } else { "not an Error".to_string() };
    let rejected = rejected("no").await.unwrap_err();
    let second = if rejected.is_error() { rejected.message() } else { "not an Error".to_string() };
    (first, second)
}

/// `x.toFixed(digits)`, as JS rounds, and `format!`'s, as Rust does.
pub fn fixed(x: f64, digits: usize) -> (String, String) {
    (js::number::to_fixed(x, digits as u32), format!("{x:.digits$}"))
}

/// `done` after `ms`.
pub fn after(ms: u32, done: Box<dyn FnOnce()>) {
    js::set_timeout(done, ms);
}

/// `never` after `ms`, cleared first: it doesn't run.
pub fn cancelled(ms: u32, never: Box<dyn FnOnce()>) {
    let id = js::set_timeout(never, ms);
    js::clear_timeout(id);
}

/// `tick` every `ms`, until `stop` of what this gives.
pub fn every(ms: u32, tick: Box<dyn FnMut()>) -> &'static js::IntervalId {
    js::set_interval(tick, ms)
}

pub fn stop(id: &js::IntervalId) {
    js::clear_interval(id);
}

async fn doubled(n: u32) -> u32 {
    n * 2
}

/// A future, unawaited, as the promise it is: what a binding taking a
/// promise is given, as react.dev's "Copy page" gives `ClipboardItem` one.
pub async fn promised(n: u32) -> u32 {
    let pending: js::Promise<u32> = js::promise(doubled(n));
    js::settle(pending).await.unwrap_or(0)
}
