// Two directories down, `./greet.js` still means the file beside the root's JS.
#[cfg_attr(rust_js, rust_js::link_name = "./greet.js#default")]
fn greet(name: &str) -> String {
    unreachable!()
}

// A file beside this one, named from the root's: `./wave.js` here.
#[cfg_attr(rust_js, rust_js::link_name = "./inner/wave.js#default")]
fn wave(name: &str) -> String {
    unreachable!()
}

// The root's import too, beside this file's own `join`: renamed here,
// and only here.
#[cfg_attr(rust_js, rust_js::link_name = "node:path#join")]
fn path_join(a: &str, b: &str) -> String {
    unreachable!()
}

// Named as the root's import of `node:path`'s `join`, which this file
// imports too: the root's keeps its name.
fn join(greeting: String) -> String {
    greeting
}

pub fn hello() -> String {
    join(greet("leaf"))
}

pub fn bye() -> String {
    wave("leaf")
}

pub fn joined() -> String {
    join(path_join("a", "leaf"))
}
