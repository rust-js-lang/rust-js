// A closure given back behind `impl Fn` is the closure its function made:
// one that can't panic is called with nothing to clean up, and one that can
// drops what's live around its call as the panic unwinds, as Rust does, an
// `Rc`'s, a `Box`'s and a plain one alike (#1142).
use std::panic;
use std::rc::Rc;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn checked(limit: u32) -> impl Fn(u32) -> u32 {
    move |x| {
        if x > limit {
            panic!("{x} is over {limit}");
        }
        x * 2
    }
}

fn shared(limit: u32) -> Rc<impl Fn(u32) -> u32> {
    Rc::new(checked(limit))
}

fn boxed(limit: u32) -> Box<impl Fn(u32) -> u32> {
    Box::new(checked(limit))
}

fn main() {
    let f = checked(5);
    let held = Loud("plain");
    println!("{}", f(2));
    let r = panic::catch_unwind(|| {
        let around = Loud("around plain");
        println!("{}", f(9));
        drop(around);
    });
    println!("{} {}", r.is_err(), held.0);

    let s = shared(5);
    let r = panic::catch_unwind(|| {
        let around = Loud("around rc");
        let other = Rc::clone(&s);
        println!("{}", other(7));
        drop(around);
    });
    println!("{} {}", r.is_err(), Rc::strong_count(&s));

    let b = boxed(5);
    let r = panic::catch_unwind(|| {
        let around = Loud("around box");
        println!("{} {}", b(1), b(8));
        drop(around);
    });
    println!("{}", r.is_err());
    println!("end");
}
