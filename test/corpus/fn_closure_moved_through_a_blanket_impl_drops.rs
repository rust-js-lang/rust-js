// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved once into a function whose `T: Thunk` is the crate's own
// trait's blanket impl for every `F: FnOnce`, which calls it by `FnOnce`
// and by nothing else (#2031): its body is its `call_once`'s, so what it
// took is dropped as its call ends, as Rust's `call_once` drops it (#995).
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
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        8
    };
    println!("{}", forced(h));
    println!("end");
}
