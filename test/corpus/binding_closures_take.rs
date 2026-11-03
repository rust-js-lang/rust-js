// A pattern's binding that closures take by value is one `const` where the
// program binds it, which each closure and what reads it after close over,
// as React's isSignatureEqual's `boundPromise` (rd640): no copy apart for
// each closure. Each closure reads what the binding took, though the field
// it was read from is given another value after.

use std::cell::Cell;

struct Closure<'a> {
    bound: Option<&'a Cell<u32>>,
}

fn callbacks<'a>(closure: &Closure<'a>, base: u32) -> Option<Vec<Box<dyn Fn(u32) -> u32 + 'a>>> {
    let Some(bound) = closure.bound else {
        return None;
    };
    match bound.get() {
        0 => Some(Vec::new()),
        _ => {
            bound.set(bound.get() + base);
            Some(vec![
                Box::new(move |v| {
                    let fulfilled: &Cell<u32> = bound;
                    fulfilled.set(v);
                    fulfilled.get()
                }),
                Box::new(move |e| {
                    let rejected: &Cell<u32> = bound;
                    rejected.set(e + 10);
                    rejected.get() * 2
                }),
            ])
        }
    }
}

fn main() {
    let first = Cell::new(3);
    let other = Cell::new(100);
    let mut closure = Closure { bound: Some(&first) };
    let made = callbacks(&closure, 4).unwrap();
    closure.bound = Some(&other);
    println!("{}", first.get());
    println!("{} {}", made[0](1), made[1](2));
    println!("{} {}", first.get(), other.get());
    println!("{}", callbacks(&Closure { bound: None }, 1).is_none());
    println!("{}", callbacks(&Closure { bound: Some(&Cell::new(0)) }, 1).map_or(9, |v| v.len()));
    println!("{}", callbacks(&closure, 1).map_or(9, |v| v.len()));
    println!("{}", other.get());
}
