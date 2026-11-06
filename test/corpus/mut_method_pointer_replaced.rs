//@ library-differs
// A `&mut self` trait method as a function pointer, of an object: the
// arrow gives it a box. A default copied into the impl of an object
// `Self` takes the box apart where it's called and changes the object in
// place, `*self = other`, `mem::take(self)` and a generic callee's
// `mem::replace` alike, so the box holds the same object once it returns
// and isn't copied back (#831). A blanket impl's generic `Self` writes the
// box's `value` itself, a new object, so it's copied back, as Rust's
// `*self = ..` replaces the caller's value.
use std::mem;

#[derive(Debug, Default)]
struct Stack {
    items: Vec<i32>,
}

fn put<T>(slot: &mut T, value: T) -> T {
    mem::replace(slot, value)
}

trait Reset: Default {
    fn reset(&mut self) -> Self {
        put(self, Self::default())
    }
    fn swap_in(&mut self, other: Self) {
        *self = other;
    }
    fn through(&mut self) -> Self {
        let again: &mut Self = self;
        mem::take(again)
    }
}

impl Reset for Stack {}

trait Renew {
    fn renew(&mut self, other: Self)
    where
        Self: Sized;
}

impl<T> Renew for T {
    fn renew(&mut self, other: T) {
        *self = other;
    }
}

trait Wipe: Default {
    fn wipe(&mut self) {
        *self = Self::default();
    }
}

impl<T: Default> Wipe for T {}

fn main() {
    let reset: fn(&mut Stack) -> Stack = Stack::reset;
    let swap_in: fn(&mut Stack, Stack) = Stack::swap_in;
    let through: fn(&mut Stack) -> Stack = Stack::through;
    let renew: fn(&mut Stack, Stack) = Stack::renew;
    let mut s = Stack { items: vec![1, 2] };
    let alias = &s as *const Stack;
    let old = reset(&mut s);
    s.items.push(3);
    println!("{:?} {:?}", old, s);
    swap_in(&mut s, Stack { items: vec![7] });
    s.items.push(8);
    println!("{:?}", s);
    let taken = through(&mut s);
    s.items.push(4);
    println!("{:?} {:?} {}", taken, s, alias == &s as *const Stack);
    renew(&mut s, Stack { items: vec![5] });
    s.items.push(6);
    println!("{:?}", s);
    s.renew(Stack { items: vec![9] });
    println!("{:?}", s);
    let wipe: fn(&mut Stack) = Stack::wipe;
    wipe(&mut s);
    s.items.push(10);
    println!("{:?}", s);
}
