//@ compile-fail: a closure that holds a value with a destructor, called by `FnOnce`, which drops none of it
// A closure rustc infers `Fn`, holding a value with a destructor, moved
// into a guard whose `Drop`, std's trait, takes it out and calls it by
// `FnOnce` (#2031): std calls the drop where the crate can't see, so the
// impl's own parameter is what's in its place in each guard the crate
// holds, `Guard<{closure}>`, and its call by value is refused where the
// closure's made, as Rust's `call_once` drops it after the call.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Guard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for Guard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f()
        }
    }
}

fn main() {
    let d = Loud("guarded");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
    };
    let _guard = Guard(Some(h));
    println!("end");
}
