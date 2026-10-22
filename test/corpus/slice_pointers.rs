// A raw pointer to a slice, `*const [T]`, is the slice itself, as a pointer
// to a JS object is the object (ADR 0285): an array, or a view of part of
// one (ADR 0335). Read through, it's the slice; its `len()` the slice's;
// cast to its item's, its first item's handle; and `slice_from_raw_parts`
// a view of as many items from an item's pointer. As arrayvec's `drain`
// keeps the slice it drains, and hashbrown's allocator gives a `NonNull<[u8]>`.
use std::ptr::{self, NonNull};

fn drained(v: &mut Vec<i32>, start: usize, end: usize) -> Vec<i32> {
    let range: *const [i32] = &v[start..end];
    let taken: Vec<i32> = unsafe { (*range).iter().copied().collect() };
    v.drain(start..end);
    taken
}

fn doubled(items: &mut [i32]) {
    let whole: *mut [i32] = items;
    let first = whole as *mut i32;
    let len = whole.len();
    for i in 0..len {
        unsafe { *first.add(i) *= 2 };
    }
}

fn tail(items: &mut [u8], from: usize) -> NonNull<[u8]> {
    let start = unsafe { NonNull::new_unchecked(items.as_mut_ptr().add(from)) };
    NonNull::slice_from_raw_parts(start, items.len() - from)
}

/// Each item read through a `*const` of the shared reference to it, as
/// arrayvec's `Drain` reads them: a cell of the value, which reads the same.
struct Taken<'a, T> {
    iter: std::slice::Iter<'a, T>,
}

impl<T> Iterator for Taken<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.iter.next().map(|elt| unsafe { ptr::read(elt as *const _) })
    }
}

/// The same of a value JS holds as itself, a number: a cell of it, which
/// reads the same, where its place has no name in JS.
fn read_all(items: &[u32]) -> Vec<u32> {
    items
        .iter()
        .map(|elt| unsafe { ptr::read(elt as *const u32) })
        .collect()
}

/// A `&mut`'s, only read through: where it is matters to nothing.
fn peeked(count: &mut u32) -> u32 {
    *count += 1;
    let p = count as *const u32;
    unsafe { ptr::read(p) }
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// Each item of a slice dropped in place through its pointer, as arrayvec's
/// `truncate` drops its tail; the vector forgotten after, its first item
/// taken out first.
fn dropped_tail() {
    let mut items = vec![Noisy(1), Noisy(2), Noisy(3)];
    unsafe {
        let tail = std::slice::from_raw_parts_mut(items.as_mut_ptr().add(1), 2);
        ptr::drop_in_place(tail);
    }
    let first = unsafe { ptr::read(&items[0]) };
    std::mem::forget(items);
    println!("{}", first.0);
}

fn main() {
    let mut v = vec![1, 2, 3, 4, 5];
    println!("{:?} {:?}", drained(&mut v, 1, 3), v);
    let mut items = [1, 2, 3];
    doubled(&mut items);
    println!("{items:?}");
    let mut bytes = [1u8, 2, 3, 4];
    let mut rest = tail(&mut bytes, 2);
    println!("{}", rest.len());
    unsafe { rest.as_mut()[0] = 9 };
    let raw = ptr::slice_from_raw_parts_mut(bytes.as_mut_ptr(), 2);
    unsafe { (*raw)[1] = 7 };
    if let Some(front) = unsafe { raw.as_mut() } {
        front[0] = 8;
    }
    println!("{bytes:?}");
    let taken: Vec<i32> = Taken { iter: [4, 5, 6].iter() }.collect();
    println!("{taken:?}");
    println!("{:?}", read_all(&[7, 8]));
    dropped_tail();
    let mut count = 4;
    println!("{} {count}", peeked(&mut count));
}
