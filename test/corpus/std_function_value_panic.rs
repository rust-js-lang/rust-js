//@ run-fail: called `Option::unwrap()` on a `None` value
//@ located-otherwise: #2335 a `#[track_caller]` std function given as a value, `.map(Option::unwrap)`, names its caller's line, where native names the `Fn` trait's shim
// `Option::unwrap` as a value panics as its call does, at the item that's `None`.
fn main() {
    let seen: Vec<i32> = vec![Some(1), None, Some(3)]
        .into_iter()
        .inspect(|o| println!("{o:?}"))
        .map(Option::unwrap)
        .collect();
    println!("{seen:?}");
}
