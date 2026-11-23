//@ run-fail: called `Option::unwrap()` on a `None` value
// `Option::unwrap` boxed, then read out of its box and given to
// `Iterator::map`: as `.map(Option::unwrap)`, it panics at
// `FnMut::call_mut`'s line (#2335).
fn main() {
    let unwrap = Box::new(Option::unwrap);
    let seen: Vec<i32> = vec![Some(1), None].into_iter().map(*unwrap).collect();
    println!("{seen:?}");
}
