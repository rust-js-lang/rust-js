//@ compile-fail: which calls it once, a function called by `&` that holds a value with a destructor
// A closure called by `&` that holds a value with a destructor, given
// through `impl Is<F>` to generic code that calls `F: FnOnce()` once, is
// refused as it is given to `F` itself: that call drops none of it, where
// Rust's drops it once it's run (ADR 0470, #966).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn run<F: FnOnce()>(f: impl Is<F>) {
    let f: F = f.is();
    f()
}

fn main() {
    let p = Loud("p");
    run(move || {
        let x = &p;
        println!("both {}", x.0);
    });
}
