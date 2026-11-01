// A struct that extends another, as React DOM's legacy Fizz config's render
// state extends the Fizz config's (ADR 0204, amended): the flattened field
// is the parent's object, lent whole to the functions of its own type,
// which read and set its fields there.
use std::cell::Cell;

pub struct RenderState {
    pub prefix: String,
    pub styles_to_hoist: Cell<bool>,
    pub written: Cell<u32>,
}

pub struct LegacyRenderState {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub base: RenderState,
    pub generate_static_markup: bool,
}

fn create_render_state(prefix: &str) -> RenderState {
    RenderState {
        prefix: prefix.to_owned(),
        styles_to_hoist: Cell::new(false),
        written: Cell::new(0),
    }
}

fn write_segment(render_state: &RenderState, id: u32) -> String {
    render_state.written.set(render_state.written.get() + 1);
    render_state.styles_to_hoist.set(true);
    format!("{}{id}", render_state.prefix)
}

fn create_legacy_render_state(prefix: &str, generate_static_markup: bool) -> LegacyRenderState {
    LegacyRenderState {
        base: create_render_state(prefix),
        generate_static_markup,
    }
}

fn write_legacy_segment(render_state: &LegacyRenderState, id: u32) -> String {
    if render_state.generate_static_markup {
        return String::new();
    }
    write_segment(&render_state.base, id)
}

fn main() {
    let state = create_legacy_render_state("S:", false);
    println!("{}", write_legacy_segment(&state, 1));
    println!("{}", write_legacy_segment(&state, 2));
    println!("{} {}", state.base.written.get(), state.base.styles_to_hoist.get());
    let quiet = create_legacy_render_state("P:", true);
    println!("[{}]", write_legacy_segment(&quiet, 3));
    println!("{} {}", quiet.base.written.get(), quiet.base.styles_to_hoist.get());
    let base = create_render_state("B:");
    println!("{} {}", write_segment(&base, 4), base.written.get());
}
