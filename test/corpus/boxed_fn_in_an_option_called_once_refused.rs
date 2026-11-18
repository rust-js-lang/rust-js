//@ compile-fail: a closure that holds a value with a destructor, made a `dyn Fn` called by `FnOnce` where nothing drops it after
// A `Box<dyn Fn()>` holding a value with a destructor, given in an
// `Option` to a generic function that calls what it holds by `FnOnce`
// (#2050): the call gives the `Option`, not the box, so it can't give the
// box as one whose call drops it (#2033). Refused where it's made.
struct Loud(&'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run<F: FnOnce() -> u32>(o: Option<F>) -> u32 {
    match o {
        Some(f) => f(),
        None => 0,
    }
}

fn main() {
    let d = Loud("in an option");
    let b: Box<dyn Fn() -> u32> = Box::new(move || {
        let held = &d;
        println!("in {}", held.0);
        3
    });
    println!("{}", run(Some(b)));
    println!("end");
}
