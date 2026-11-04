// What generic code moves out of an object's `&mut T` it's given, other
// than by `mem::replace` of the parameter itself: through a struct that
// holds the `&mut`, a closure that captures it, `mem::take`, `mem::swap`
// with a local, into a `Vec` it's given, and through a trait's impl of a
// concrete type. Each is a copy apart where it's moved out (`moved_out`),
// so the caller's box holds the object itself, and the object, written
// back in place, is the new value (ADR 0454, amended).
use std::mem;

#[derive(Debug, Default, Clone)]
struct Stack {
    items: Vec<i32>,
}

struct Slot<'a, T> {
    r: &'a mut T,
}

impl<'a, T> Slot<'a, T> {
    fn put(&mut self, v: T) -> T {
        mem::replace(self.r, v)
    }
}

fn through_struct<T>(r: &mut T, v: T) -> T {
    let mut slot = Slot { r };
    slot.put(v)
}

fn through_closure<T>(r: &mut T, v: T) -> T {
    let mut put = |x: T| mem::replace(&mut *r, x);
    put(v)
}

fn through_default<T: Default>(r: &mut T) -> T {
    mem::take(r)
}

fn through_swap<T>(r: &mut T, mut v: T) -> T {
    mem::swap(r, &mut v);
    v
}

fn into_vec<T>(r: &mut T, v: T, out: &mut Vec<T>) {
    out.push(mem::replace(r, v));
}

trait Reset: Sized {
    fn reset(&mut self) -> Self;
}

impl Reset for Stack {
    fn reset(&mut self) -> Self {
        mem::take(self)
    }
}

fn through_trait<T: Reset>(r: &mut T) -> T {
    r.reset()
}

fn main() {
    let mut s = Stack { items: vec![1] };
    let old = through_struct(&mut s, Stack { items: vec![2] });
    s.items.push(3);
    println!("{s:?} {old:?}");

    let mut s = Stack { items: vec![1] };
    let old = through_closure(&mut s, Stack { items: vec![2] });
    s.items.push(3);
    println!("{s:?} {old:?}");

    let mut s = Stack { items: vec![1] };
    let old = through_default(&mut s);
    s.items.push(3);
    println!("{s:?} {old:?}");

    let mut s = Stack { items: vec![1] };
    let old = through_swap(&mut s, Stack { items: vec![2] });
    s.items.push(3);
    println!("{s:?} {old:?}");

    let mut s = Stack { items: vec![1] };
    let mut out = Vec::new();
    into_vec(&mut s, Stack { items: vec![2] }, &mut out);
    s.items.push(3);
    println!("{s:?} {out:?}");

    let mut s = Stack { items: vec![1] };
    let old = through_trait(&mut s);
    s.items.push(3);
    println!("{s:?} {old:?}");

    let mut v = vec![1, 2];
    let old = through_struct(&mut v, vec![9]);
    v.push(3);
    println!("{v:?} {old:?}");
}
