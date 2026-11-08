//@ compile-fail: a `#[rust_js::nullable]` field of an `Option` of an `Option` is left out already for `None`
// `#[rust_js::optional]` of a nullable field of an `Option` of an `Option`,
// whose `None` is left out already and its `Some(None)` `null` (ADR 1017):
// refused with what's so of it, where the refusal named a nullable field
// as what it needs, which this one is (#959).
#[derive(Debug)]
pub struct Member {
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub value: Option<Option<i32>>,
}

fn main() {
    println!("{:?}", Member { value: Some(None) });
}
