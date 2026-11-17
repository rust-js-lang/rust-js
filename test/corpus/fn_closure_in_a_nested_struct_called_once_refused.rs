//@ compile-fail: a closure that holds a value with a destructor, moved where it's called by `FnOnce`, which drops none of it
// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved into a struct held in a field of another, `W { o:
// Once(h) }`, which a function takes and calls that closure of by `FnOnce`
// alone, `(w.o.0)()` of a `W<F>` of an `F: FnOnce` (#2031): refused, as its
// call drops none of it, however deep the struct holding it is.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Once<F>(F);

struct W<F> {
    o: Once<F>,
}

fn run<F: FnOnce() -> u32>(w: W<F>) -> u32 {
    (w.o.0)()
}

fn main() {
    let d = Loud("nested");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        7
    };
    println!("{}", run(W { o: Once(h) }));
}
