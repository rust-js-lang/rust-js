// A nullable `Option<Option<T>>` field, ADR 1017's held form, of a class of
// no fields: its value is the instance its constructor made, never
// `undefined` or `null`, so its `Some(Some(m))` is `m` itself (#832, ADR
// 1041, amended), as `may_look_like_none` says, where the refusal's own
// catalog took it for a unit struct, which is `undefined`.

#[cfg_attr(rust_js, rust_js::class)]
pub struct Marker {}

#[cfg_attr(rust_js, rust_js::class)]
impl Marker {
    pub fn new() -> Self {
        Marker {}
    }

    pub fn name(&self) -> &'static str {
        "marker"
    }
}

struct Holder {
    #[cfg_attr(rust_js, rust_js::nullable)]
    link: Option<Option<Marker>>,
}

fn show(h: &Holder) {
    match &h.link {
        None => println!("none"),
        Some(None) => println!("some none"),
        Some(Some(m)) => println!("some some {}", m.name()),
    }
}

fn main() {
    show(&Holder { link: Some(Some(Marker::new())) });
    show(&Holder { link: Some(None) });
    show(&Holder { link: None });
}
