//@ compile-fail: a closure that holds a value with a destructor, made a `dyn Fn` called by `FnOnce` where nothing drops it after
// A `Box<dyn Fn()>` holding a value with a destructor, given in a `Vec` to
// a generic function that calls each item by `FnOnce` (#2050): the call
// gives the `Vec`, not the box, so it can't give the box as one whose call
// drops it (#2033). Refused where it's made.
struct Loud(&'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run_all<F: FnOnce() -> u32>(v: Vec<F>) -> u32 {
    v.into_iter().map(|f| f()).sum()
}

fn main() {
    let d = Loud("in a vec");
    let b: Box<dyn Fn() -> u32> = Box::new(move || {
        let held = &d;
        println!("in {}", held.0);
        3
    });
    println!("{}", run_all(vec![b]));
    println!("end");
}
