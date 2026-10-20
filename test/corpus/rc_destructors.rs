// An `Rc` of a value with a destructor is counted, as one whose counts are
// read is (ADR 0320): the last of it drops what it points at, where Rust's
// does, through a clone, a `Vec` of them, a `RefCell` in it, one a
// function makes and gives back, and one of a value that holds such an
// `Rc`, which drops because that one is counted.
use std::cell::RefCell;
use std::rc::Rc;

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Holder {
    name: Noisy,
    items: Vec<Noisy>,
}

struct Outer {
    inner: Rc<Noisy>,
}

fn shared(name: &'static str) -> Rc<Noisy> {
    let made = Rc::new(Noisy(name));
    let kept = made.clone();
    println!("made {}", kept.0);
    made
}

fn main() {
    let a = Rc::new(Noisy("a"));
    {
        let b = Rc::clone(&a);
        println!("{}", b.0);
    }
    println!("a still {}", a.0);
    let c = a.clone();
    drop(a);
    println!("c {}", c.0);
    drop(c);
    println!("after a");

    let h = Rc::new(Holder { name: Noisy("h"), items: vec![Noisy("i1"), Noisy("i2")] });
    let v = vec![h.clone(), h.clone()];
    drop(h);
    println!("{} {}", v.len(), v[0].name.0);
    drop(v);
    println!("after h");

    let cell = Rc::new(RefCell::new(vec![Noisy("x")]));
    cell.borrow_mut().push(Noisy("y"));
    let other = cell.clone();
    drop(cell);
    println!("{}", other.borrow().len());
    let last = other.borrow_mut().pop();
    println!("popped {}", last.is_some());
    drop(last);
    drop(other);

    let outer = Rc::new(Outer { inner: Rc::new(Noisy("o")) });
    let again = outer.clone();
    drop(outer);
    println!("outer kept {}", again.inner.0);
    drop(again);

    let s = shared("s");
    println!("got {}", s.0);
    println!("end");
}
