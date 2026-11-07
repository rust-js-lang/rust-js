// `dedup_by` and `dedup_by_key` drop each duplicate as the closure says
// it is one, before the next call, of any item: a `Box`'s and an `Rc`'s,
// each given to the closure as a `&mut` to it is, written back as it
// returns or panics; and `pop_if` of `Rc`s, its closure given the same
// (#906).
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 5 {
            panic!("drop");
        }
    }
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("Box, by key", || {
        let mut v = vec![Box::new(D(3)), Box::new(D(1)), Box::new(D(4)), Box::new(D(2))];
        v.dedup_by_key(|x| {
            println!("key {}", x.0);
            x.0 % 2
        });
        println!("{}", v.len());
    });
    run("Rc", || {
        let mut v = vec![Rc::new(D(3)), Rc::new(D(1)), Rc::new(D(4)), Rc::new(D(2))];
        v.dedup_by(|a, b| {
            println!("by {} {}", a.0, b.0);
            a.0 % 2 == b.0 % 2
        });
        println!("{}", v.len());
    });
    run("Rc, a drop panics", || {
        let mut v = vec![Rc::new(D(3)), Rc::new(D(5)), Rc::new(D(4)), Rc::new(D(6))];
        v.dedup_by(|a, b| a.0 % 2 == b.0 % 2);
        println!("{}", v.len());
    });
    run("Box, written then a panic", || {
        let mut v = vec![Box::new(D(3)), Box::new(D(1)), Box::new(D(4))];
        let r = catch_unwind(AssertUnwindSafe(|| {
            v.dedup_by(|a, b| {
                if a.0 == 4 {
                    *a = Box::new(D(7));
                    panic!("by");
                }
                a.0 % 2 == b.0 % 2
            })
        }));
        println!(
            "{} {}",
            r.is_ok(),
            v.iter().map(|d| d.0.to_string()).collect::<Vec<_>>().join(" ")
        );
    });
    run("pop_if of Rcs", || {
        let mut v = vec![Rc::new(D(3)), Rc::new(D(1))];
        let popped = v.pop_if(|x| {
            println!("pop_if {}", x.0);
            x.0 == 1
        });
        println!("{} {}", popped.is_some(), v.len());
    });
    run("pop_if of Boxes, written then a panic", || {
        let mut v = vec![Box::new(D(3)), Box::new(D(1))];
        let r = catch_unwind(AssertUnwindSafe(|| {
            v.pop_if(|x| {
                *x = Box::new(D(6));
                panic!("pop_if");
            })
        }));
        println!("{} {}", r.is_ok(), v[1].0);
    });
}
