// A `Box<dyn Fn()>` whose closure holds a `Box<dyn Any>` of a value with a
// destructor drops it as the `dyn Fn` drops: the `dyn Fn` drops only once
// `dyn Any`s are known to, which were settled after it, once, so it dropped
// nothing (#1662). No closure here holds a value with a destructor itself,
// which would make every `dyn Fn()` drop.
use std::any::Any;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run(f: &dyn Fn()) {
    f();
}

fn main() {
    let held: Box<dyn Any> = Box::new(Loud("a"));
    let f: Box<dyn Fn()> = Box::new(move || {
        let _ = &held;
        println!("called");
    });
    f();
    run(&*f);
    drop(f);
    println!("after a");

    let mut fs: Vec<Box<dyn Fn()>> = Vec::new();
    for name in ["b", "c"] {
        let held: Box<dyn Any> = Box::new(Loud(name));
        fs.push(Box::new(move || println!("is loud {}", held.is::<Loud>())));
    }
    fs.iter().for_each(|f| f());
    drop(fs.remove(0));
    println!("removed");
}
