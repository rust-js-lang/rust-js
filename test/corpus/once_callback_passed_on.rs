// A closure that only passes its call on to a function of the crate's,
// given to a binding's `FnOnce` callback, is that function (ADR 0262's
// exception): JS calls it once, with what the binding gives it, which the
// function reads no more of than the closure gives it. A function of rest
// arguments reads all JS gives it, so the closure that gives it one of
// them stays an arrow, an array method's callback too (ADR 0279, amended).
// A listener, an `Fn`'s, keeps its arrow where it passes on what it's
// given, as JS may keep and compare it and give it more; one that ignores
// what it's given is the function it calls (ADR 0605, amended).

fn count() -> u32 {
    println!("count");
    7
}

fn shown(x: u32) -> u32 {
    x + 1
}

fn many(#[cfg_attr(rust_js, rust_js::rest)] xs: &[u32]) -> u32 {
    xs.len() as u32
}

/// `callback.call(undefined, value)`: JS calls the callback once, with
/// `value`, as `then` does with what it fulfils.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn once_with(this: Box<dyn FnOnce(u32) -> u32>, this_arg: (), value: u32) -> u32 {
    let () = this_arg;
    this(value)
}

/// `callback.call(undefined, a, b)`: once, with two.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn once_with_two(this: Box<dyn FnOnce(u32, u32) -> u32>, this_arg: (), a: u32, b: u32) -> u32 {
    let () = this_arg;
    this(a, b)
}

/// `callback.call()`: once, with nothing, as `setTimeout` does.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn once(this: impl FnOnce() -> u32) -> u32 {
    this()
}

/// `listener.call(undefined, value)`: a listener, which JS may keep and
/// call again.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn listened(this: Box<dyn Fn(u32) -> u32>, this_arg: (), value: u32) -> u32 {
    let () = this_arg;
    this(value)
}

fn main() {
    println!("{}", once_with(Box::new(|_| count()), (), 5));
    println!("{}", once_with(Box::new(|x| shown(x)), (), 5));
    println!("{}", once_with_two(Box::new(|x, _| shown(x)), (), 5, 6));
    println!("{}", once_with_two(Box::new(|x, _| many(&[x])), (), 5, 6));
    println!("{}", once(|| count()));
    println!("{}", listened(Box::new(|_| count()), (), 5));
    println!("{}", listened(Box::new(|x| shown(x)), (), 5));
    let mapped: Vec<u32> = [4, 5].iter().map(|x| many(&[*x])).collect();
    println!("{mapped:?}");
}
