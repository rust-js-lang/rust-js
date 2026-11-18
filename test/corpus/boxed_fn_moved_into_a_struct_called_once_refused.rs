//@ compile-fail: a closure that holds a value with a destructor, made a `dyn Fn` called by `FnOnce` where nothing drops it after
// A `Box<dyn Fn()>` holding a value with a destructor, moved into a struct
// whose trait impl calls it by `FnOnce`, through a generic caller (#2050):
// the call gives the struct, not the box, so it can't give the box as one
// whose call drops it (#2033). Refused where it's made.
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
    let d = Loud("boxed in a struct");
    let b: Box<dyn Fn() -> u32> = Box::new(move || {
        let held = &d;
        println!("in {}", held.0);
        4
    });
    println!("{}", run_it(Once(b)));
    println!("end");
}
