//@ library-refused: another crate may share
// A clone of a closure holding a counted `Rc` holds a clone of it, which
// it drops on its own; one that changes what it took changes its own copy;
// a `T: Clone` clones it too, and each drops its own (ADR 0522).
use std::rc::Rc;

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Tag(u32);

impl Clone for Tag {
    fn clone(&self) -> Tag {
        Tag(self.0 + 10)
    }
}

impl Drop for Tag {
    fn drop(&mut self) {
        println!("drop tag {}", self.0);
    }
}

#[derive(Clone)]
struct Holder<F> {
    f: F,
}

fn pair<F: Fn() -> u32 + Clone>(f: F) -> (F, F) {
    (f.clone(), f)
}

fn main() {
    let shared = Rc::new(Noisy(7));
    let read = {
        let shared = Rc::clone(&shared);
        move || shared.0
    };
    let read2 = read.clone();
    println!("{} {} count {}", read(), read2(), Rc::strong_count(&shared));
    drop(read);
    println!("count {}", Rc::strong_count(&shared));

    let mut n = 0;
    let mut step = move || {
        n += 1;
        n
    };
    let mut step2 = step.clone();
    println!("{} {} {}", step(), step(), step2());

    let doubled = {
        let shared = Rc::clone(&shared);
        move || shared.0 * 2
    };
    let (a, b) = pair(doubled);
    println!("{} count {}", a() + b(), Rc::strong_count(&shared));
    drop((a, b));
    let holder = Holder { f: read2 };
    let held = vec![holder.clone(), holder];
    println!("{} count {}", (held[0].f)(), Rc::strong_count(&shared));
    drop(held);
    println!("count {}", Rc::strong_count(&shared));
    drop(shared);

    let tag = Tag(1);
    let show = move || {
        let t = &tag;
        t.0
    };
    let show2 = show.clone();
    println!("tags {} {}", show(), show2());
    drop(show);
    println!("end");
}
