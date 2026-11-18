//@ compile-fail: a closure that holds a value with a destructor, called by `FnOnce`, which drops none of it
// A closure rustc infers `Fn`, holding a value with a destructor, made
// where it's given to a function whose `T: Thunk` is the crate's own
// trait's blanket impl for every `F: FnOnce`, which calls it by `FnOnce`
// (#2031): the impl's own parameter is what the trait's method's namings
// give, so its call by value is seen, and refused where the closure's
// made, as Rust's `call_once` drops it after the call and JS's doesn't.
struct Loud(&'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Thunk {
    fn force(self) -> u32;
}

impl<F: FnOnce() -> u32> Thunk for F {
    fn force(self) -> u32 {
        self()
    }
}

fn forced<T: Thunk>(t: T) -> u32 {
    t.force()
}

fn main() {
    let d = Loud("blanket");
    println!(
        "{}",
        forced(move || {
            let held = &d;
            println!("in {}", held.0);
            8
        })
    );
    println!("end");
}
