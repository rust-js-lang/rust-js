// An `Rc` of a `&mut` where the crate counts every `Rc`: an impl of a
// trait for any `Rc<T>`, beside an `Rc<dyn Fn>` whose `ptr_eq` it asks of
// a cloned closure (ADR 0852, ADR 0959). `{}` and `{:?}` of it show what
// the `&mut` borrows, read through the `&mut`'s cell where it's one, a
// number's or a `String`'s, as an uncounted `Rc`'s are, a value written
// through it before it's shown too (ADR 0504, #1577).
use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// # Safety
/// Its target stays put while it's moved.
pub unsafe trait StableDeref: Deref {}

unsafe impl<T: ?Sized> StableDeref for Rc<T> {}

fn total<D: StableDeref<Target = Vec<u32>>>(items: D) -> u32 {
    items.iter().sum()
}

fn cloned(count: &Rc<RefCell<u32>>) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
    let count = Rc::clone(count);
    let read = move || *count.borrow();
    (Rc::new(read.clone()), Rc::new(read))
}

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
    println!("{}", total(Rc::new(vec![5, 6])));
    let held = Rc::new(RefCell::new(4));
    let (first, second) = cloned(&held);
    println!("{} {} {}", first(), second(), Rc::ptr_eq(&first, &second));

    let mut count = 7u32;
    count += 1;
    {
        let shared = Rc::new(&mut count);
        println!("{shared} {shared:?} {}", shared.to_string());
    }
    {
        let mut written = Rc::new(&mut count);
        if let Some(through) = Rc::get_mut(&mut written) {
            **through += 10;
        }
        println!("{written} {written:?}");
    }
    println!("{count}");
    {
        let boxed = Box::new(&mut count);
        println!("{boxed} {boxed:?}");
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
