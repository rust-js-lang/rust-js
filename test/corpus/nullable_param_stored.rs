// ADR 0861, amended: a function's own nullable parameter is `null` or a
// value only where each of its calls gives it so. One a call may give
// what's `undefined`, the `None` of a plain `Option`, `find`'s say, or a
// call through a `fn` pointer, is stored `null` for `None`: a nullable
// field of a struct only the crate makes is tested `=== null`, so a `None`
// stored `undefined` read as a `Some`. One each call gives `null` or a
// value is stored as it is.
struct Holder<'a> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    config: Option<&'a str>,
}

#[cfg_attr(rust_js, rust_js::nullable(config))]
fn held(config: Option<&str>) -> bool {
    let holder = Holder { config };
    holder.config.is_some()
}

fn given(config: Option<&str>) -> bool {
    held(config)
}

#[cfg_attr(rust_js, rust_js::nullable(config))]
fn pointed(config: Option<&str>) -> bool {
    let holder = Holder { config };
    holder.config.is_some()
}

fn pointer() -> fn(Option<&str>) -> bool {
    pointed
}

#[cfg_attr(rust_js, rust_js::nullable(config))]
fn kept(config: Option<&str>) -> bool {
    let holder = Holder { config };
    holder.config.is_some()
}

fn main() {
    let words = ["a", "b"];
    let missing = words.iter().copied().find(|w| *w == "z");
    let found = words.iter().copied().find(|w| *w == "b");
    println!("{} {}", given(missing), given(found));
    println!("{} {}", pointer()(missing), pointer()(found));
    println!("{} {}", kept(None), kept(Some("a")));
}
