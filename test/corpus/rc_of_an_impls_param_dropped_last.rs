// An `Rc` of an impl's own type parameter, which its dictionary gives, so
// no caller shows what it may be (ADR 0852), made of a value whose drop
// runs: here through a `dyn` call. The drop given that parameter makes the
// `Rc` counted (ADR 0914, amended), so the value is dropped by its last
// clone, not its first, as native Rust drops it (#1009, #1112).
use std::rc::Rc;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Keep {
    fn keep(self: Box<Self>);
}

struct Wrap<T>(T);

impl<T> Keep for Wrap<T> {
    fn keep(self: Box<Self>) {
        let first = Rc::new(self.0);
        let second = Rc::clone(&first);
        drop(first);
        println!("still held");
        drop(second);
        println!("kept");
    }
}

fn main() {
    let kept: Box<dyn Keep> = Box::new(Wrap(Loud(1)));
    kept.keep();
    println!("end");
}
