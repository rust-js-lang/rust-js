//! Imports from JS modules: `#[link_name = "module#path"]` (ADR 0028).

pub mod inner {
    pub mod leaf;
}

/// A URL, as JS has it: a JS type is a struct of a `JsObject` (ADR 0111).
struct Url(core::marker::PhantomData<js::JsObject>);

unsafe extern "Rust" {
    #[link_name = "./greet.js#punctuation"]
    safe static punctuation: &'static str;
}

// Named imports, and a path inside one.
#[cfg_attr(rust_js, rust_js::link_name = "node:path#join")]
fn path_join(a: &str, b: &str) -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "node:path#posix.basename")]
fn basename(path: &str) -> String {
    unreachable!()
}

// A class from a module, next to the global of the same name.
#[cfg_attr(rust_js, rust_js::link_name = "new node:url#URL")]
fn new_url(href: &str) -> &'static Url {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new URL")]
fn new_global_url(href: &str) -> &'static Url {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get href")]
fn href(this: &Url) -> String {
    unreachable!()
}

// A file of our own, beside the root's JS: its default export, and a
// path through the whole module.
#[cfg_attr(rust_js, rust_js::link_name = "./greet.js#default")]
fn greet(name: &str) -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./greet.js#*.polite")]
fn polite(name: &str) -> String {
    unreachable!()
}

// Another module's export named as an import of this one, `greet` beside
// `./greet.js`'s default, is imported apart from it.
#[cfg_attr(rust_js, rust_js::link_name = "./inner/wave.js#greet")]
fn wave(name: &str) -> String {
    unreachable!()
}

pub fn paths() -> String {
    // A local named like an import doesn't hide it.
    let join = "c.txt";
    path_join(&path_join("a", "b"), join)
}

pub fn waved() -> String {
    wave("root")
}

pub fn file_name() -> String {
    basename("/tmp/x/y.txt")
}

pub fn urls() -> (String, String) {
    (href(new_url("https://example.com/a")), href(new_global_url("https://example.com/b")))
}

pub fn greetings() -> (String, String, &'static str) {
    (greet("world"), polite("world"), punctuation)
}
