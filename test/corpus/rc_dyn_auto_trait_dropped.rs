// A counted `Rc<dyn Sub + Send>` as an `Rc<dyn Sub>`, an auto trait left
// out (#186): the same `Rc`, its counts shared.
use std::rc::Rc;

trait Sub {
    fn f(&self) -> u32;
}

struct S(u32);

impl Sub for S {
    fn f(&self) -> u32 {
        self.0
    }
}

fn main() {
    let a: Rc<dyn Sub + Send> = Rc::new(S(3));
    let b: Rc<dyn Sub> = a.clone();
    println!("{} {} {}", b.f(), Rc::strong_count(&a), Rc::strong_count(&b));
    drop(a);
    println!("{}", Rc::strong_count(&b));
}
