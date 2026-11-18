//@ compile-fail: a closure that holds a value with a destructor, called by `FnOnce`, which drops none of it
// A closure rustc infers `Fn`, holding a value with a destructor, called by
// `&` before it's moved into a generic function that calls it by `FnOnce`
// (#995): its body can't drop what it took as it ends, which its first call
// would, so it's refused, as one not moved there alone is.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn call_once<F: FnOnce() -> u32>(f: F) -> u32 {
    f()
}

fn main() {
    let d = Loud("called first");
    let f = move || {
        let held = &d;
        println!("in {}", held.0);
        1
    };
    println!("{}", f());
    println!("{}", call_once(f));
}
