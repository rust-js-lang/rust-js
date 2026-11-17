//@ compile-fail: a closure that holds a value with a destructor, moved where it's called by `FnOnce`, which drops none of it
// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved into a struct a free generic function takes, `Once<F>`
// of an `F: FnOnce`, and calls that field of by `FnOnce` alone (#2031):
// refused, as its call drops none of it.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Once<F>(F);

fn run<F: FnOnce() -> u32>(o: Once<F>) -> u32 {
    (o.0)()
}

fn main() {
    let d = Loud("through a function");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        5
    };
    println!("{}", run(Once(h)));
}
