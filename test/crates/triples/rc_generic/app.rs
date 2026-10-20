// An `Rc<T>` a generic library makes, of a value with a destructor, held by
// another crate's trait object, callback and `Rc`, and changed in place
// (ADR 0523).
use std::fmt;
use std::rc::Rc;

struct Loud(u32);

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Loud({})", self.0)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

pub fn main() {
    use mid::Show;
    let loud = dep::share(Loud(1));
    let shown = mid::boxed(Rc::clone(&loud));
    let counter = mid::callback(Rc::clone(&loud));
    println!("{} {} {}", shown.show(), counter(), dep::owners(&loud));
    drop(shown);
    println!("{}", counter());
    let nested = mid::nest(Rc::clone(&loud));
    println!("{} {}", Rc::strong_count(&nested), dep::owners(&loud));
    drop(nested);
    drop(counter);
    println!("{}", dep::owners(&loud));
    drop(loud);

    let mut v = dep::share(vec![1u32]);
    let other = Rc::clone(&v);
    dep::bump(&mut v);
    println!("{:?} {:?} {}", v, other, dep::owners(&v));
    let mut n = dep::share(1u32);
    println!("{} {}", dep::set_if_alone(&mut n, 5), n);
    let keep = Rc::clone(&n);
    println!("{} {} {}", dep::set_if_alone(&mut n, 7), n, keep);
}
