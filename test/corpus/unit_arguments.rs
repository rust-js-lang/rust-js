// A `()` given a function value is no argument, `f()` of `f(())`, as JS
// leaves one out (ADR 0441): a closure's, a `dyn Fn`'s, one after another
// argument. One whose making does something is still made, first.
fn apply(f: &dyn Fn(()), g: &dyn Fn(u32, ())) {
    f(());
    g(1, ());
}

fn noted() {
    println!("made the unit");
}

fn each<T: Copy>(f: impl Fn(T), value: T) {
    f(value);
    f(value);
}

fn main() {
    let h = |_: ()| println!("h");
    h(());
    apply(&h, &|n, _| println!("g {n}"));
    let k = |_: ()| println!("k");
    k(noted());
    each(|_: ()| println!("each"), ());
    each(|n: u8| println!("each {n}"), 3);
}
