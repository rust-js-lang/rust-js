// Three crates share one `Rc`: cloned in one, counted in another, its last
// owner dropped in each, and its `Weak` upgraded before and after (ADR
// 0523); and one `dep` counts none of, an `Rc<String>`, the value itself
// in each (ADR 0847).
use std::rc::Rc;

pub fn main() {
    let a = dep::make("a");
    let weak = dep::watch(&a);
    let mut holder = mid::Holder { items: Vec::new() };
    holder.keep(&a);
    println!("{} {} {}", Rc::strong_count(&a), dep::count(&a), dep::alive(&weak));
    drop(a);
    let last = holder.release().unwrap();
    dep::finish(last);
    println!("alive {}", dep::alive(&weak));

    let b = dep::make("b");
    let (b1, b2) = mid::twice(b);
    println!("{}", Rc::strong_count(&b1));
    drop(b1);
    mid::drop_here(b2);

    let c = dep::make("c");
    let w = Rc::downgrade(&c);
    holder.keep(&c);
    drop(holder);
    println!("{} {}", Rc::strong_count(&c), w.upgrade().is_some());
    drop(c);
    println!("{} {}", w.upgrade().is_none(), dep::alive(&w));

    let tag = dep::tag("t");
    let again = Rc::clone(&tag);
    println!("{tag} {again}");
}
