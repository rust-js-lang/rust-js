//@ compile-fail: a closure that holds a value with a destructor, moved where it's called by `FnOnce`, which drops none of it
// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved into a struct whose trait impl calls it by `FnOnce`
// alone (#2031): Rust's `call_once` of it drops it after the call, which
// JS's call of its body doesn't, and the struct's drop doesn't run of a
// field moved out. Refused, as the same closure made in place is.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Run {
    fn run(self) -> u32;
}

struct Once<F>(F);

impl<F: FnOnce() -> u32> Run for Once<F> {
    fn run(self) -> u32 {
        (self.0)()
    }
}

fn run_it<R: Run>(r: R) -> u32 {
    r.run()
}

fn main() {
    let d = Loud("through a trait");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        4
    };
    println!("{}", run_it(Once(h)));
}
