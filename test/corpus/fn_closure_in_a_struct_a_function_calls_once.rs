// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved into a struct a free generic function takes, `Once<F>`
// of an `F: FnOnce`, and calls that field of by `FnOnce` alone (#2031):
// every call of it is by value, so its body is its `call_once`'s, dropping
// what it took as that call ends (#2600).
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
    let d = Loud("through a function");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        5
    };
    println!("{}", run(Once(h)));
}
