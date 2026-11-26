// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved into a struct held in a field of another, `W { o:
// Once(h) }`, which a function takes and calls that closure of by `FnOnce`
// alone, `(w.o.0)()` of a `W<F>` of an `F: FnOnce` (#2031): however deep
// the struct holding it, every call of it is by value, so its body is its
// `call_once`'s, dropping what it took as that call ends (#2600).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Once<F>(F);

struct W<F> {
    o: Once<F>,
}

fn run<F: FnOnce() -> u32>(w: W<F>) -> u32 {
    (w.o.0)()
}

fn main() {
    let d = Loud("nested");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        7
    };
    println!("{}", run(W { o: Once(h) }));
}
