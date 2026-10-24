// A `Box` or an `Rc` of a `&mut`, which is a handle on what it borrows,
// shows what it borrows, through the handle, as `{}` and `{:?}` of the
// `&mut` itself do (ADR 0023, #197); a counted `Rc`'s too (ADR 0504).
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl fmt::Display for Loud {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "loud {}", self.0)
    }
}

fn main() {
    let mut count = 7u32;
    count += 1;
    {
        let boxed = Box::new(&mut count);
        println!("{boxed} {boxed:?}");
    }
    {
        let shared = Rc::new(&mut count);
        println!("{shared} {shared:?} {}", shared.to_string());
    }
    let mut name = String::from("a");
    name.push('b');
    let named = Rc::new(&mut name);
    println!("{named} {named:?} {}", named.len());

    let mut loud = Loud(3);
    loud.0 += 1;
    let counted = Rc::new(&mut loud);
    let again = Rc::clone(&counted);
    println!("{counted} {again:?} {}", Rc::strong_count(&counted));
}
