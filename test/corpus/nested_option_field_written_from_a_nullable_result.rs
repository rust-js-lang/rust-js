// A nullable field of an `Option` of an `Option`, held as its payload (ADR
// 1017: `None` absent, `Some(None)` `null`), written from the crate's own
// `#[rust_js::nullable]` function, whose `None` is `null`: its `None` is
// the field's `None`, not `Some(None)`, as a `None` of any form is told by
// `== null` where its `Some`, boxed, never is (#957).
#[derive(Debug)]
struct Held {
    #[cfg_attr(rust_js, rust_js::nullable)]
    v: Option<Option<i32>>,
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn given(v: Option<Option<i32>>) -> Option<Option<i32>> {
    v
}

fn main() {
    let mut h = Held { v: Some(Some(1)) };
    h.v = given(None);
    println!("{:?} {}", h.v, h.v.is_none());
    h.v = given(Some(None));
    println!("{:?}", h);
    h.v = given(Some(Some(3)));
    println!("{:?}", h);
}
