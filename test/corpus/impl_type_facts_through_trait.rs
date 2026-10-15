// A trait impl's method that asks `size_of::<T>()`, called through its
// trait from another impl's, of a type naming the caller's `T`: arrayvec's
// `FromIterator::from_iter` calling `array.extend(iter)`. The call is its
// impl's, so the caller asks it too, and passes its own on (ADR 0145).
// And one given its impl's dictionary for a bound, `collect()`'s `B:
// FromIterator<T>` of a `Stack<T>`, arrayvec's `Clone`, or a generic
// function's own `C: Push<T>`, asks it too.
use std::mem;

struct Stack<T> {
    items: Vec<T>,
    sized: usize,
}

impl<T> Extend<T> for Stack<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            if mem::size_of::<T>() != 0 {
                self.sized += 1;
            }
            self.items.push(item);
        }
    }
}

impl<T> FromIterator<T> for Stack<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut stack = Stack { items: Vec::new(), sized: 0 };
        stack.extend(iter);
        stack
    }
}

impl<T: Clone> Clone for Stack<T> {
    fn clone(&self) -> Self {
        self.items.iter().cloned().collect()
    }
}

trait Push<T> {
    fn put(&mut self, item: T);
}

impl<T> Push<T> for Stack<T> {
    fn put(&mut self, item: T) {
        self.extend([item]);
    }
}

fn twice<C: Push<T>, T: Clone>(into: &mut C, item: T) {
    into.put(item.clone());
    into.put(item);
}

fn fill<T: Clone>(stack: &mut Stack<T>, item: T) {
    twice(stack, item);
}

fn main() {
    let s: Stack<u8> = [1, 2, 3].into_iter().collect();
    let u: Stack<()> = [(), ()].into_iter().collect();
    println!("{} {} {} {}", s.items.len(), s.sized, u.items.len(), u.sized);
    let (mut s, mut u) = (s.clone(), u.clone());
    fill(&mut s, 4);
    fill(&mut u, ());
    println!("{} {} {} {}", s.items.len(), s.sized, u.items.len(), u.sized);
}
