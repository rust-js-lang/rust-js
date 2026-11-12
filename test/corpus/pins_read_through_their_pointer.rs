// A `Pin` of a pointer other than a reference or a `Box`, read and written
// through as std's are: of the crate's own smart pointer, `as_ref()`, `*p`
// and its fields through its `deref()`, `as_mut()` and `set(v)` through its
// `deref_mut()`, each called as often as natively, of a struct and of a
// number; `Rc::pin` and `Arc::pin`, an `Rc` made, a counted one's `value`
// read; `map_unchecked`; a `ManuallyDrop`, what it holds, as a `Box` is.
// Each was refused (#1448, #1454).
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
struct Key(u32);

/// A pointer of the crate's own, which says when it's read.
struct Tagged {
    key: Box<Key>,
    count: u32,
}

impl Deref for Tagged {
    type Target = Key;
    fn deref(&self) -> &Key {
        println!("deref");
        &self.key
    }
}

impl DerefMut for Tagged {
    fn deref_mut(&mut self) -> &mut Key {
        println!("deref_mut");
        self.count += 1;
        &mut self.key
    }
}

/// One whose target is a number.
struct Counter(Box<u32>);

impl Deref for Counter {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}

impl DerefMut for Counter {
    fn deref_mut(&mut self) -> &mut u32 {
        &mut self.0
    }
}

fn main() {
    let mut p = Pin::new(Tagged {
        key: Box::new(Key(7)),
        count: 0,
    });
    println!("{} {:?}", p.0, *p.as_ref());
    p.as_mut().0 += 1;
    p.set(Key(40));
    println!("{:?}", *p);
    println!("{}", Pin::into_inner(p).count);
    let mut c = Pin::new(Counter(Box::new(3)));
    *c.as_mut() += 4;
    c.set(10);
    println!("{} {}", *c, *c.as_ref() + 1);
    let r = Rc::pin(Key(1));
    let shared = Pin::clone(&r);
    println!("{} {:?}", r.0, *shared);
    println!("{}", Rc::strong_count(&Pin::into_inner(shared)));
    let a = Arc::pin(String::from("arc"));
    println!("{} {}", a.len(), a.as_ref().to_uppercase());
    let mapped = unsafe { r.as_ref().map_unchecked(|k| &k.0) };
    println!("{}", *mapped);
    let mut kept = Pin::new(ManuallyDrop::new(vec![1u8]));
    kept.as_mut().push(2);
    println!("{:?}", *kept);
    kept.set(vec![7, 8, 9]);
    println!("{:?} {}", *kept, kept.as_ref().len());
    let mut n = Pin::new(ManuallyDrop::new(5u32));
    *n.as_mut() += 1;
    n.set(*n + 34);
    println!("{} {}", *n, *n.as_ref() + 2);
}
