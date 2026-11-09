//! A library's generic code given values to drop that its own crate's
//! calls never gave it, which its consumer compiles itself where it gives
//! them (ADR 0914), and a type of its own whose drop panics, which its
//! consumer's own code drops as Rust does (#1031).
use std::ptr;

pub struct Loud(pub u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// An iterator of no destructor whose items have one.
pub struct Gen(pub u8);

impl Iterator for Gen {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        if self.0 < 4 { Some(Loud(self.0)) } else { None }
    }
}

/// Its first item dropped, its second given back.
pub fn second<I: Iterator>(mut iter: I) -> Option<I::Item> {
    iter.next();
    iter.next()
}

/// `T` dropped only as `M`'s items, through `second`.
pub fn made_second<T, M: Iterator<Item = T>>(made: M) -> Option<T> {
    second(made)
}

/// A closure holding what it's given, made a `dyn Fn` by `boxed`.
pub fn boxed<F: Fn() -> usize + 'static>(f: F) -> Box<dyn Fn() -> usize> {
    Box::new(f)
}

pub fn holding<K: 'static>(value: K) -> Box<dyn Fn() -> usize> {
    let shared = vec![value];
    boxed(move || shared.len())
}

/// Its items dropped in place through a raw pointer.
pub struct Slots<T> {
    pub items: Vec<T>,
}

impl<T> Slots<T> {
    pub fn truncate(&mut self, len: usize) {
        while self.items.len() > len {
            unsafe {
                let last = self.items.len() - 1;
                self.items.set_len(last);
                ptr::drop_in_place(self.items.as_mut_ptr().add(last));
            }
        }
    }
}

/// What `at` points to, dropped there: given nothing but the pointer,
/// which owns none.
///
/// # Safety
/// `at` points to a value nothing drops again.
pub unsafe fn drop_at<T>(at: *mut T) {
    unsafe { ptr::drop_in_place(at) }
}

/// A drop that panics on its third item.
pub struct Fragile(pub u8);

impl Drop for Fragile {
    fn drop(&mut self) {
        println!("drop fragile {}", self.0);
        if self.0 == 3 {
            panic!("fragile");
        }
    }
}
