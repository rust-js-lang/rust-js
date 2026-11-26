// A closure rustc infers `Fn`, holding a value with a destructor, moved
// into a guard whose `Drop`, std's trait, takes it out and calls it by
// `FnOnce` (#2031): the impl's own parameter is what's in its place in
// each guard the crate holds, `Guard<{closure}>`, and every call of it is
// by value, so its body is its `call_once`'s, dropping what it took as the
// guard's drop calls it, as Rust's `call_once` drops it (#2600).
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
