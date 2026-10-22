// A closure returned as `impl FnMut` is the function, as one made here is:
// lent by `&mut` to generic code or to a `&mut dyn FnMut`, it's called
// there, and keeps counting where it's called again.
fn counter() -> impl FnMut() -> u32 {
    let mut n = 0;
    move || {
        n += 1;
        n
    }
}

fn twice<F: FnMut() -> u32>(mut f: F) -> u32 {
    f() + f()
}

fn once(f: &mut dyn FnMut() -> u32) -> u32 {
    f()
}

fn main() {
    let mut count = counter();
    println!("{}", twice(&mut count));
    println!("{}", once(&mut count));
    println!("{}", count());
}
