// An `Rc` of a closure that holds a value with a destructor, given back by
// its function as `Rc<impl Fn>`, owns the closure: what the closure took is
// dropped once, at the last clone's drop, not where the `Rc` is made (#464).
// A generic function's is the same, and a `Box<impl Fn>`'s, and so are the
// `Rc<dyn Fn>` and the local `Rc` that were right before.
use std::rc::Rc;

struct Manifest(u32);

impl Drop for Manifest {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

fn plain(value: Manifest) -> Rc<impl Fn(u32) -> u32> {
    Rc::new(move |x| {
        let held = &value;
        x + held.0
    })
}

fn hold<T: 'static>(value: T) -> Rc<impl Fn(u32) -> u32> {
    Rc::new(move |x| {
        let _held = &value;
        x + 1
    })
}

fn in_a_box(value: Manifest) -> Box<impl Fn(u32) -> u32> {
    Box::new(move |x| {
        let held = &value;
        x + held.0
    })
}

fn boxed(value: Manifest) -> Rc<dyn Fn(u32) -> u32> {
    Rc::new(move |x| {
        let held = &value;
        x + held.0
    })
}

fn main() {
    let p = plain(Manifest(2));
    let q = Rc::clone(&p);
    drop(p);
    println!("plain {}", q(1));
    drop(q);

    let h = hold(Manifest(3));
    let k = Rc::clone(&h);
    drop(h);
    println!("hold {}", k(1));
    drop(k);

    let b = in_a_box(Manifest(6));
    println!("box {}", b(1));
    drop(b);

    let d = boxed(Manifest(4));
    let e = Rc::clone(&d);
    drop(d);
    println!("dyn {}", e(1));
    drop(e);

    let m = Manifest(5);
    let local = Rc::new(move |x: u32| {
        let held = &m;
        x + held.0
    });
    let again = Rc::clone(&local);
    drop(local);
    println!("local {}", again(1));
    drop(again);
    println!("end");
}
