// A `#[rust_js::nullable]` parameter of an `Option` of a value never `null`
// that may be `undefined`, rd645's `ServerManifest`: given `Some` from
// Rust, each read of it in the callee, `unwrap_or`, `if let`, `match`,
// `is_some`, a call it's given on to, a plain `Option` it's stored in, sees
// `Some` (#2267).
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Manifest {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Undefined,
}

static UNDEFINED: Manifest = Manifest::Undefined;

#[cfg_attr(rust_js, rust_js::nullable(m))]
fn unwrap_or(m: Option<&'static Manifest>) -> bool {
    m.copied().unwrap_or(Manifest::Undefined) == Manifest::Undefined && m.is_some()
}

#[cfg_attr(rust_js, rust_js::nullable(m))]
fn if_let(m: Option<&'static Manifest>) -> &'static str {
    if let Some(_) = m { "some" } else { "none" }
}

#[cfg_attr(rust_js, rust_js::nullable(m))]
fn matched(m: Option<&'static Manifest>) -> u32 {
    match m {
        Some(Manifest::Undefined) => 1,
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(m))]
fn is_some(m: Option<&'static Manifest>) -> bool {
    m.is_some()
}

fn plain(m: Option<&'static Manifest>) -> bool {
    m.is_some()
}

#[cfg_attr(rust_js, rust_js::nullable(m))]
fn given_on(m: Option<&'static Manifest>) -> bool {
    plain(m)
}

struct Kept {
    m: Option<&'static Manifest>,
}

#[cfg_attr(rust_js, rust_js::nullable(m))]
fn stored(m: Option<&'static Manifest>) -> bool {
    let kept = Kept { m };
    kept.m.is_some()
}

fn pick(some: bool) -> Option<&'static Manifest> {
    if some { Some(&UNDEFINED) } else { None }
}

fn main() {
    for some in [true, false] {
        println!(
            "{} {} {} {} {} {}",
            unwrap_or(pick(some)),
            if_let(pick(some)),
            matched(pick(some)),
            is_some(pick(some)),
            given_on(pick(some)),
            stored(pick(some)),
        );
        println!("{} {}", is_some(Some(&UNDEFINED)), is_some(None));
    }
}
