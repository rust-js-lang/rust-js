// Generic code given a counted `Rc`, one whose count is read, as its
// `K: Borrow<Q>` or `T: AsRef<U>`: what it borrows as is what the `Rc`
// points at, its `value`, not the `Rc` (#1503), as a known type's
// `rc.borrow()` is (ADR 0503); an `Rc` not counted is its value already,
// and a `Box` of a counted one is it.
use std::borrow::Borrow;
use std::rc::Rc;

fn len<K: Borrow<str>>(k: K) -> usize {
    k.borrow().len()
}

fn shout<T: AsRef<str>>(t: T) -> String {
    t.as_ref().to_uppercase()
}

fn first<T: AsRef<[i32]>>(t: &T) -> i32 {
    t.as_ref()[0]
}

fn value_of<K: Borrow<i32>>(k: &K) -> i32 {
    *k.borrow() + 1
}

fn main() {
    let shared: Rc<str> = Rc::from("abc");
    let other = Rc::clone(&shared);
    println!("{} {}", Rc::strong_count(&shared), len(Rc::clone(&shared)));
    println!("{}", shout(other));
    let boxed: Box<Rc<str>> = Box::new(Rc::clone(&shared));
    println!("{} {}", len(boxed.as_ref().clone()), shout(*boxed));

    let xs: Rc<[i32]> = Rc::from(vec![4, 5]);
    let ys = Rc::clone(&xs);
    println!("{} {}", Rc::strong_count(&xs), first(&ys));

    let n = Rc::new(41);
    let m = Rc::clone(&n);
    println!("{} {}", Rc::strong_count(&n), value_of(&m));

    // Not counted: its value already.
    let plain: Rc<[i32]> = Rc::from(vec![7]);
    println!("{}", first(&plain));
}
