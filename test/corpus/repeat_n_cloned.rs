// A `repeat_n`'s clone, std's derived `Clone`: its count copied and its
// value cloned while it has one left, none once it's done. Each of the
// two then gives its own items, a clone of its own value each step but
// its last, which is the value itself, and drops what it holds as std's
// `RepeatN` does: before its first step, part way, done, of a count of
// none, and dropped unstepped (#1579).
use std::iter::repeat_n;

#[derive(Debug)]
struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Loud {
        println!("clone {}", self.0);
        Loud(self.0 + 10)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let a = repeat_n(5u8, 3);
    let b = a.clone();
    println!("{:?} {:?}", a.collect::<Vec<_>>(), b.collect::<Vec<_>>());

    println!("-- part way");
    let mut c = repeat_n(Loud(1), 3);
    let first = c.next();
    println!("first {first:?}");
    let d = c.clone();
    println!("{:?}", c.collect::<Vec<_>>());
    println!("{:?}", d.collect::<Vec<_>>());

    println!("-- done");
    let mut e = repeat_n(Loud(2), 1);
    let only = e.next();
    let f = e.clone();
    println!("{only:?} {} {}", e.count(), f.count());

    println!("-- none");
    let g = repeat_n(Loud(3), 0);
    let h = g.clone();
    println!("{} {}", g.len(), h.len());

    println!("-- dropped unstepped");
    let i = repeat_n(Loud(4), 2);
    let j = i.clone();
    drop(i);
    println!("last {:?}", j.last());

    println!("-- strings");
    let mut k = repeat_n(String::from("x"), 2);
    let next = k.next();
    let l = k.clone();
    println!("{next:?} {:?} {:?}", k.collect::<Vec<_>>(), l.collect::<Vec<_>>());
}
