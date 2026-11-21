//@ library-differs
// `Rc::map(this, f)` and `Arc::map`, Rust 1.100's: `f` of what it points
// at, `this` dropped after, or as `f` panics, and what `f` gives in an
// `Rc` of its own; counted or not, generic, chained, with a `Weak` left
// that no longer upgrades, as native's (#1656).
use std::rc::{Rc, Weak};
use std::sync::Arc;

struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn paired<T: Clone>(r: Rc<T>) -> Rc<(T, T)> {
    Rc::map(r, |t| (t.clone(), t.clone()))
}

fn main() {
    let r = Rc::new(7u32);
    let new = Rc::map(r, |i| i + 7);
    println!("{new}");
    let a = Rc::new(L(1));
    let b = Rc::map(a, |l| L(l.0 + 10));
    println!("mapped {}", b.0);
    let shared = Rc::new(L(2));
    let keep = Rc::clone(&shared);
    let c = Rc::map(shared, |l| l.0 * 3);
    println!("{} {} {}", c, keep.0, Rc::strong_count(&keep));
    let a = Rc::new(L(4));
    let w: Weak<L> = Rc::downgrade(&a);
    let d = Rc::map(a, |l| l.0 + 1);
    println!("{} {}", d, w.upgrade().is_none());
    let p = paired(Rc::new(String::from("x")));
    println!("{} {}", p.0, p.1);
    let q = paired(Rc::new(6u8));
    println!("{:?}", *q);
    let s = Arc::new(String::from("hi"));
    let len = Arc::map(s, |s| s.len());
    println!("{len}");
    let arc = Arc::new(L(5));
    let arc2 = Arc::map(arc, |l| L(l.0 * 2));
    println!("arc {}", arc2.0);
    let n = Rc::map(Rc::map(Rc::new(1i64), |x| x - 3), |x| x.abs());
    println!("{n}");
    let r = std::panic::catch_unwind(|| {
        let p = Rc::new(L(3));
        Rc::map(p, |l| -> u32 {
            if l.0 == 3 { panic!("boom") } else { l.0 }
        })
    });
    println!("{}", r.is_err());
    println!("end");
}
