// A `Vec`'s `clone_from` is std's `SpecCloneIntoVec`: what it has past
// the source's length dropped, each item it keeps given its own
// `clone_from` of the source's, a clone then the old one's drop where it
// writes none, then the rest cloned onto it; so are a `Vec`'s items' as
// `fill` and `clone_from_slice` write them over (#919).
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

struct D(u32);

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        if self.0 == 4 {
            panic!("clone");
        }
        D(self.0)
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 8 {
            panic!("drop");
        }
    }
}

#[derive(Debug)]
struct F(u32);

impl Clone for F {
    fn clone(&self) -> F {
        println!("clone F {}", self.0);
        F(self.0)
    }

    fn clone_from(&mut self, source: &F) {
        println!("clone_from F {} {}", self.0, source.0);
        self.0 = source.0;
    }
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("longer", || {
        let v = vec![D(3), D(1), D(2)];
        let mut w = vec![D(7)];
        w.clone_from(&v);
        println!("{}", w.len());
    });
    run("shorter", || {
        let v = vec![D(3)];
        let mut w = vec![D(7), D(6), D(9)];
        w.clone_from(&v);
        println!("{}", w.len());
    });
    run("a clone panics", || {
        let v = vec![D(3), D(1), D(4), D(2)];
        let mut w = vec![D(7)];
        w.clone_from(&v);
        println!("{}", w.len());
    });
    run("a drop panics", || {
        let v = vec![D(3), D(1)];
        let mut w = vec![D(8), D(7)];
        w.clone_from(&v);
        println!("{}", w.len());
    });
    run("of Vecs", || {
        let v = vec![vec![D(3), D(1)], vec![D(2)]];
        let mut w = vec![vec![D(7)], vec![D(6), D(9)], vec![]];
        w.clone_from(&v);
        println!("{}", w.len());
    });
    run("fill of Vecs", || {
        let mut w = vec![vec![D(7)], vec![D(6), D(9)], vec![D(5)]];
        w.fill(vec![D(3), D(1)]);
        println!("{}", w.len());
    });
    run("clone_from_slice of Vecs", || {
        let v = vec![vec![D(3), D(1)], vec![D(2)]];
        let mut w = vec![vec![D(7)], vec![D(6), D(9)]];
        w.clone_from_slice(&v);
        println!("{}", w.len());
    });
    run("of Rcs", || {
        let v = vec![Rc::new(D(3)), Rc::new(D(1))];
        let mut w = vec![Rc::new(D(7)), Rc::new(D(6)), Rc::new(D(9))];
        w.clone_from(&v);
        println!("{}", w.len());
    });
    run("its items' own clone_from", || {
        let v = vec![F(1), F(2), F(3)];
        let mut w = vec![F(7), F(6)];
        w.clone_from(&v);
        println!("{w:?}");
        let mut x = vec![vec![F(5)], vec![]];
        x.fill(vec![F(6), F(4)]);
        println!("{x:?}");
    });
    run("of what drops nothing", || {
        let s = vec![String::from("a"), String::from("b")];
        let mut t = vec![String::from("c")];
        t.clone_from(&s);
        let n = vec![1, 2, 3];
        let mut m = vec![4];
        m.clone_from(&n);
        println!("{t:?} {m:?}");
    });
}
