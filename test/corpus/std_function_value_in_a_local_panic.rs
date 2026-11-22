//@ run-fail: called `Option::unwrap()` on a `None` value
// `Option::unwrap` put in a local, then given to `Iterator::map`, its one use:
// as `.map(Option::unwrap)`, it panics at `FnMut::call_mut`'s line (#2335).
fn main() {
    let unwrap = Option::unwrap;
    let seen: Vec<i32> = vec![Some(1), None].into_iter().map(unwrap).collect();
    println!("{seen:?}");
}
