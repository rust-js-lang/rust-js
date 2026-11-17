//@ compile-fail: giving `via`, which calls it once, a function called by `&` that holds a value with a destructor
// A closure rustc infers `Fn`, holding a value with a destructor, given to
// a function whose parameter is an `impl Wrapped<I>` of an `I: Is<F>` of
// an `F: FnOnce`, two bounds from the `FnOnce` that calls it (#2031): that
// call drops none of it, so it's refused, as one bound deep is (#966).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Is<T> {
    fn get(self) -> T;
}

impl<T> Is<T> for T {
    fn get(self) -> T {
        self
    }
}

trait Wrapped<T> {
    fn unwrapped(self) -> T;
}

impl<T> Wrapped<T> for T {
    fn unwrapped(self) -> T {
        self
    }
}

fn via<I: Is<F>, F: FnOnce() -> u32>(i: impl Wrapped<I>) -> u32 {
    i.unwrapped().get()()
}

fn main() {
    let d = Loud("two bounds deep");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        6
    };
    println!("{}", via(h));
}
