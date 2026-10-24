// A counted `Rc<dyn Sub>` as an `Rc<dyn Super>` (#186): the supertrait's
// pair, whose value and counts are the `Rc`'s own, shared: a clone, a
// drop, `strong_count`, a `Weak` and `ptr_eq` through either are the same
// allocation's, two levels of supertraits too.
use std::rc::{Rc, Weak};
trait Base {
    fn id(&self) -> u32;
}
trait Named: Base {
    fn name(&self) -> String;
}
trait Animal: Named {
    fn legs(&self) -> u32;
}
struct Bird(u32);
impl Base for Bird {
    fn id(&self) -> u32 {
        self.0
    }
}
impl Named for Bird {
    fn name(&self) -> String {
        format!("bird{}", self.0)
    }
}
impl Animal for Bird {
    fn legs(&self) -> u32 {
        2
    }
}
impl Drop for Bird {
    fn drop(&mut self) {
        println!("drop bird{}", self.0);
    }
}

fn show(n: Rc<dyn Named>) -> String {
    format!("{} {}", n.name(), Rc::strong_count(&n))
}

fn main() {
    let a: Rc<dyn Animal> = Rc::new(Bird(1));
    let n1: Rc<dyn Named> = a.clone();
    let n2: Rc<dyn Named> = a.clone();
    println!("{} {} {}", Rc::ptr_eq(&n1, &n2), Rc::strong_count(&a), n1.id());
    let b: Rc<dyn Base> = n1.clone();
    println!("{} {} {}", b.id(), Rc::strong_count(&n2), show(n2));
    let w: Weak<dyn Named> = Rc::downgrade(&n1);
    println!(
        "{} {}",
        Rc::weak_count(&a),
        w.upgrade().map(|n| n.name()).unwrap_or_default()
    );
    drop(n1);
    drop(b);
    println!("{}", Rc::strong_count(&a));
    drop(a);
    println!("{}", w.upgrade().is_none());
    let solo: Rc<dyn Animal> = Rc::new(Bird(2));
    let up: Rc<dyn Named> = solo;
    println!("{} {}", up.name(), Rc::strong_count(&up));
    println!("end");
}
