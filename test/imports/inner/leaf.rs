// Two directories down, `./greet.js` still means the file beside the root's JS.
unsafe extern "Rust" {
    #[link_name = "./greet.js#default"]
    safe fn greet(name: &str) -> String;
    // A file beside this one, named from the root's: `./wave.js` here.
    #[link_name = "./inner/wave.js#default"]
    safe fn wave(name: &str) -> String;
}

// Named as the root's import of `node:path`'s `join`, which this file
// doesn't import: the root's keeps its name.
fn join(greeting: String) -> String {
    greeting
}

pub fn hello() -> String {
    join(greet("leaf"))
}

pub fn bye() -> String {
    wave("leaf")
}
