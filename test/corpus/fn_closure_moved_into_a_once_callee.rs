// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved, its only use, into a generic function that calls it by
// `FnOnce` alone (#995): Rust's `call_once` of it calls it by `&` and then
// drops it, so what it took is dropped as that call ends, or as it
// unwinds, or where the function never calls it, as it drops `f`. Through
// a second generic function it's given on to, and given as an `I: Is<F>`
// of an `F: FnOnce`, whose value it is (#966). A shadow of its name, a
// second `f`, is another variable, each moved once.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn call_once<F: FnOnce() -> u32>(f: F) -> u32 {
    f()
}

fn maybe<F: FnOnce() -> u32>(f: F, go: bool) -> u32 {
    if go { f() } else { 0 }
}

fn forward<F: FnOnce() -> u32>(f: F) -> u32 {
    println!("forwarding");
    call_once(f)
}

trait Is<T> {
    fn get(self) -> T;
}

impl<T> Is<T> for T {
    fn get(self) -> T {
        self
    }
}

fn via<I: Is<F>, F: FnOnce() -> u32>(i: I) -> u32 {
    i.get()()
}

fn main() {
    let a = Loud("called");
    let f = move || {
        let held = &a;
        println!("in {}", held.0);
        1
    };
    println!("{}", call_once(f));

    let b = Loud("uncalled");
    let f = move || {
        let held = &b;
        println!("in {}", held.0);
        2
    };
    println!("{}", maybe(f, false));

    let c = Loud("forwarded");
    let g = move || {
        let held = &c;
        println!("in {}", held.0);
        3
    };
    println!("{}", forward(g));

    let d = Loud("given as what it is");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        4
    };
    println!("{}", via(h));

    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let e = Loud("panicking");
        let p = move || -> u32 {
            let held = &e;
            println!("in {}", held.0);
            panic!("boom");
        };
        call_once(p)
    }));
    println!("unwound {}", unwound.is_err());
    println!("end");
}
