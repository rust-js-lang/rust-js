//! A closure rustc infers `Fn`, holding a value with a destructor, moved
//! into the library's struct and given to its generic code, whose impl
//! calls it by `FnOnce` (#2609).
use dep::{Once, run_it};

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let d = Loud("through a library's trait");
    let h = move || {
        let held = &d;
        println!("in {}", held.0);
        4
    };
    println!("{}", run_it(Once(h)));
}
