// The playground, in Rust (ADR 0032): React components (ADR 0044), one per
// file in components/. rust-js compiles them to JSX beside them, running as
// WebAssembly, the same compiler the page runs (see ../compile-rust.ts), and
// main.ts calls `start`.

#![allow(non_snake_case)]

mod components;

// What the components use: loading and running the compiler, the Result
// frame's page, CodeMirror, and the crate being edited.
mod codemirror;
// Also called by compiler-worker.js through the generated module's public API.
pub mod compiler;
mod dark_mode;
mod listen;
mod programs;
mod projects;
mod styles;
mod tree;

use react::dom::client::create_root;
use webapi::document;

use components::app::App;
use react::jsx;

/// Render the page into `#app`.
pub fn start() {
    let root = create_root(document.get_element_by_id("app").expect("the page has an #app"));
    root.render(jsx! { <StrictMode><App /></StrictMode> });
}
