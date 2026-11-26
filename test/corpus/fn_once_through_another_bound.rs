// A closure rustc infers `Fn`, holding a value with a destructor, made
// where it's given through `impl Is<F>` to generic code that calls `F:
// FnOnce()` once (ADR 0470, #966): every call of it is by value, so its
// body is its `call_once`'s, dropping what it took as that call ends, as
// Rust's `call_once` drops it (#2600).
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
