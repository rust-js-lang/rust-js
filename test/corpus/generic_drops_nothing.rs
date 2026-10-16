// What std does to a generic `T`'s items in place, or adds clones of, which
// drops none of them: generic code a caller gives a type with a destructor
// (ADR 0190). `swap`, `get_mut`, `get_unchecked_mut`, `extend_from_slice`,
// `set_len`, and a generic index's `index_mut`.
use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

#[derive(Clone)]
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn shuffle<T: Clone>(v: &mut Vec<T>, extra: &[T]) -> usize {
    v.swap(0, 1);
    v.extend_from_slice(extra);
    if let Some(first) = v.get_mut(0) {
        let _ = first;
    }
    let _second = unsafe { v.get_unchecked_mut(1) };
    v.len()
}

/// The last item moved out, as `pop` does.
fn take_last<T>(v: &mut Vec<T>) -> Option<T> {
    let len = v.len();
    if len == 0 {
        return None;
    }
    unsafe {
        v.set_len(len - 1);
        Some(std::ptr::read(v.as_ptr().add(len - 1)))
    }
}

struct Wrap<T>(Vec<T>);

impl<T, I: SliceIndex<[T]>> Index<I> for Wrap<T> {
    type Output = I::Output;
    fn index(&self, index: I) -> &I::Output {
        &self.0[index]
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for Wrap<T> {
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        &mut self.0[index]
    }
}

fn main() {
    let mut v = vec![Loud("a"), Loud("b")];
    let n = shuffle(&mut v, &[Loud("c")]);
    println!("{n} {}", v.iter().map(|l| l.0).collect::<String>());
    let last = take_last(&mut v);
    println!("took {}", last.as_ref().map_or("none", |l| l.0));
    let mut w = Wrap(v);
    w[0].0 = "z";
    println!("{}", w[0..2].iter().map(|l| l.0).collect::<String>());
}
