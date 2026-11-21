// A `Pin<Box<T>>` from its `Box<T>`, std's `From`, `Pin::from(b)` or
// `b.into()`, is the box, as `Box::into_pin` is: what it holds read and
// changed through it, a `dyn`'s method called, and what it holds dropped
// where the pin is, in Rust's order (#1452).
use std::pin::Pin;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Speak {
    fn speak(&self) -> String;
}

impl Speak for Loud {
    fn speak(&self) -> String {
        format!("loud {}", self.0)
    }
}

fn pinned(b: Box<Loud>) -> Pin<Box<Loud>> {
    b.into()
}

fn main() {
    let pinned_num: Pin<Box<u32>> = Pin::from(Box::new(7));
    println!("{}", *pinned_num);

    let mut counter: Pin<Box<u32>> = Box::new(1).into();
    *counter.as_mut() += 41;
    println!("{}", *counter);

    let a = Pin::from(Box::new(Loud(1)));
    let b = pinned(Box::new(Loud(2)));
    println!("{} {}", a.0, b.0);
    drop(a);
    println!("after a");

    let spoken: Pin<Box<dyn Speak>> = Pin::from(Box::new(Loud(3)) as Box<dyn Speak>);
    println!("{}", spoken.speak());

    let inner = Pin::into_inner(Pin::from(Box::new(Loud(4))));
    println!("{}", inner.0);
    println!("end");
}
