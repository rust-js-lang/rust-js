// A struct made of constants, promoted where it's borrowed, of nullable
// fields held in a form of their own: a nested one's `Some(None)` is
// `null` (ADR 1017) and a value's `Some(())` is `undefined`, its `None`
// `null` (rd645), as where it's made at run time. A constant's value would
// be each `Some`'s box, `{ $someNone: 0 }`, which reads as `Some(Some(..))`
// (#832).

struct Holder {
    #[cfg_attr(rust_js, rust_js::nullable)]
    link: Option<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    seen: Option<()>,
}

fn show(h: &Holder) {
    let link = match &h.link {
        None => "none".to_string(),
        Some(None) => "some none".to_string(),
        Some(Some(m)) => format!("some some {m}"),
    };
    println!("{link} {:?}", h.seen);
}

fn main() {
    show(&Holder { link: Some(Some(3)), seen: Some(()) });
    show(&Holder { link: Some(None), seen: None });
    show(&Holder { link: None, seen: Some(()) });
}
