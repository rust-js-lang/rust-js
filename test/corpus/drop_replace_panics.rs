//@ ignore-rust-js: THIR's lowering writes the new value only if the old one's drop returns, so unwinding drops the old one again
//@ run-fail: boom
// An assignment drops the old value, then writes the new one: if the old
// one's drop panics, the new one is written still, and it's what unwinding
// drops after, not the old one again.
struct Loud(&'static str, bool);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.1 {
            panic!("boom");
        }
    }
}

struct Wrapper<T> {
    inner: T,
}

impl<T> Wrapper<T> {
    fn replace(&mut self, value: T) {
        self.inner = value;
        println!("replaced");
    }
}

fn main() {
    let mut wrapper = Wrapper { inner: Loud("old", true) };
    wrapper.replace(Loud("new", false));
    println!("unreached");
}
