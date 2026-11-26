//@ compile-fail: a closure that holds a value with a destructor, called by `FnOnce`, which drops none of it
// A closure rustc infers `Fn`, holding a value with a destructor, moved
// into a struct and called by `&` through it, `(o.0)()`, before a generic
// function takes the struct and calls it by `FnOnce` (#2600): its body
// can't be its `call_once`'s, as its first call would drop what it took,
// so it's refused, as the same closure called before it's moved is.
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
fn main() {
    let a = Loud("peeked");
    let h = move || {
        let held = &a;
        println!("in {}", held.0);
        1
    };
    let o = Once(h);
    println!("peek {}", (o.0)());
    println!("{}", run(o));
    println!("end");
}
