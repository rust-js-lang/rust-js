// Closures rustc infers `Fn`, each holding a value with a destructor, bound
// by `let` and moved into what's made, a struct, a tuple, an array, an
// `Option` or a struct of two, which generic code takes apart and calls by
// `FnOnce` alone (#2600): every call of each is by value, so its body is
// its `call_once`'s, dropping what it took as that call ends or unwinds;
// one never called, left in what's dropped, is dropped there, once.
use std::panic::{AssertUnwindSafe, catch_unwind};
struct Loud(&'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Once<F>(F);
impl<F: FnOnce() -> u32> Once<F> {
    fn maybe(self, go: bool) -> u32 {
        println!("maybe {go}");
        if go { (self.0)() } else { 0 }
    }
}
fn tuple<F: FnOnce() -> u32>(t: (F, u32)) -> u32 {
    (t.0)() + t.1
}
fn array<F: FnOnce() -> u32>(a: [F; 1]) -> u32 {
    let [f] = a;
    println!("between");
    f()
}
fn option<F: FnOnce() -> u32>(o: Option<F>) -> u32 {
    match o {
        Some(f) => f(),
        None => 0,
    }
}
struct Two<F, G> {
    first: F,
    second: G,
}
fn first_only<F: FnOnce() -> u32, G: FnOnce() -> u32>(t: Two<F, G>) -> u32 {
    let Two { first, second } = t;
    let r = first();
    println!("second left");
    drop(second);
    r
}
fn main() {
    let a = Loud("not called");
    let h = move || {
        let held = &a;
        println!("in {}", held.0);
        1
    };
    println!("{}", Once(h).maybe(false));
    let b = Loud("called");
    let h = move || {
        let held = &b;
        println!("in {}", held.0);
        2
    };
    println!("{}", Once(h).maybe(true));
    let c = Loud("tuple");
    let h = move || {
        let held = &c;
        println!("in {}", held.0);
        3
    };
    println!("{}", tuple((h, 10)));
    let d = Loud("array0");
    let h0 = move || {
        let held = &d;
        println!("in {}", held.0);
        4
    };
    println!("{}", array([h0]));
    let f = Loud("option");
    let h = move || {
        let held = &f;
        println!("in {}", held.0);
        6
    };
    println!("{}", option(Some(h)));
    let g1 = Loud("first");
    let g2 = Loud("second");
    let h0 = move || {
        let held = &g1;
        println!("in {}", held.0);
        7
    };
    let h1 = move || {
        let held = &g2;
        println!("in {}", held.0);
        8
    };
    println!("{}", first_only(Two { first: h0, second: h1 }));
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let p = Loud("panicking");
        let h = move || -> u32 {
            let held = &p;
            println!("in {}", held.0);
            panic!("boom");
        };
        Once(h).maybe(true)
    }));
    println!("unwound {}", unwound.is_err());
    println!("end");
}
