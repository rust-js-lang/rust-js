//! A crate given a library's counted `Rc<Slot<_>>`s, which it counts as the
//! library does, its own `Loud` dropped by the last of them, and its
//! uncounted context, the value itself in both (ADR 0852).
use std::rc::Rc;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let context = dep::create_context(1);
    println!("{} {}", dep::use_context(&context), dep::use_context(&context));
    let again = Rc::clone(&context);
    again.value.set(2);
    println!("{} {}", context.value.get(), Rc::ptr_eq(&context, &again));
    let (a, b) = dep::share(Loud("slot"));
    println!("{} {}", a.value.0, Rc::strong_count(&a));
    drop(a);
    println!("one left, {}", Rc::strong_count(&b));
    drop(b);
    let (c, d) = dep::share(3);
    println!("{} {}", c.value + d.value, Rc::strong_count(&d));
    let shelf = dep::Shelf {
        items: vec![Rc::new(dep::Item { value: 7 })],
    };
    println!("{} {}", dep::shelved(&shelf), shelf.items[0].value);
    println!("end");
}
