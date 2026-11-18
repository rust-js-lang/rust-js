//@ compile-fail: a closure that holds a value with a destructor, made a `dyn Fn` called by `FnOnce` where nothing drops it after
// A `Box<dyn Fn()>` holding a value with a destructor, given to a generic
// function that gives it back in a struct, which another calls the field of
// by `FnOnce` (#2050): no call gives the box as itself where it's called by
// value, so none gives it as one whose call drops it (#2033), and Rust's
// `call_once` of it drops it: refused where it's made.
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

fn unbounded<T>(t: T) -> Once<T> {
    Once(t)
}

fn main() {
    let e = Loud("unbounded");
    let c: Box<dyn Fn() -> u32> = Box::new(move || {
        let held = &e;
        println!("in {}", held.0);
        4
    });
    println!("{}", run(unbounded(c)));
    println!("end");
}
