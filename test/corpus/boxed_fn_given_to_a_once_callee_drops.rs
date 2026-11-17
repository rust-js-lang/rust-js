// A `Box<dyn Fn()>` or a `Box<dyn FnMut()>` whose closure holds a value with
// a destructor, given to a generic function that calls it by `FnOnce`
// alone (#2033): std's `call_once` of a `Box<F>` calls it and then drops
// the box, so what it took is dropped as that call returns, or as it
// unwinds, or where the function never calls it; given on through a second
// generic function, it's dropped once, where it's called.
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

fn boxed(name: &'static str, n: u32) -> Box<dyn Fn() -> u32> {
    let e = Loud(name);
    Box::new(move || {
        let held = &e;
        println!("in {}", held.0);
        n
    })
}

fn main() {
    println!("{}", call_once(boxed("returns", 1)));
    println!("{}", maybe(boxed("uncalled", 2), false));
    println!("{}", forward(boxed("forwarded", 3)));

    let e = Loud("mutable");
    let mut count = 0;
    let m: Box<dyn FnMut() -> u32> = Box::new(move || {
        let held = &e;
        count += 1;
        println!("in {} {count}", held.0);
        count
    });
    println!("{}", call_once(m));

    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let e = Loud("panicking");
        let p: Box<dyn Fn() -> u32> = Box::new(move || -> u32 {
            let held = &e;
            println!("in {}", held.0);
            panic!("boom");
        });
        call_once(p)
    }));
    println!("unwound {}", unwound.is_err());
    println!("end");
}
