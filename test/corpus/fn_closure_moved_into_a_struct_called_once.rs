// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved into a struct whose trait impl calls it by `FnOnce`
// alone, through a generic caller (#2031): every call of it is by value,
// as the impl's own parameter matched in `Once<F>` says, so its body is
// its `call_once`'s, and what it took is dropped as that call ends, as
// Rust's `call_once` drops it (#2600).
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
