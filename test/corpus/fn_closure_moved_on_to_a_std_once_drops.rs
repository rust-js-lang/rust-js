// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved once into a generic function that gives it on to std's
// `Option::map`, its `F: FnOnce` given an `F: FnOnce` (#2031): std's call
// by value of it is the one call, so its body is its `call_once`'s and
// drops what it took as the call ends, as Rust's `call_once` does.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn mapped<F: FnOnce(u32) -> u32>(f: F) -> Option<u32> {
    Some(1).map(f)
}

fn main() {
    let d = Loud("given on");
    let h = move |x| {
        let held = &d;
        println!("in {}", held.0);
        x + 1
    };
    println!("{:?}", mapped(h));
    println!("end");
}
