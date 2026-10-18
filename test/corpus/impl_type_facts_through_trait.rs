// A trait impl's method that asks `size_of::<T>()`, called through its
// trait from another impl's, of a type naming the caller's `T`: arrayvec's
// `FromIterator::from_iter` calling `array.extend(iter)`. The call is its
// impl's, so the caller asks it too, and passes its own on (ADR 0145).
// And one given its impl's dictionary for a bound, `collect()`'s `B:
// FromIterator<T>` of a `Stack<T>`, arrayvec's `Clone`, or a generic
// function's own `C: Push<T>`, asks it too. And a std trait's method that's
// no dictionary's, `From::from`, smallvec's `SmallVec::from(slice)`, or
// one a std method calls, `parse()`'s `FromStr::from_str`: its impl's,
// whose facts its caller asks.
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

impl<T: Clone> From<&[T]> for Stack<T> {
    fn from(items: &[T]) -> Self {
        let sized = if mem::size_of::<T>() == 0 { 0 } else { items.len() };
        Stack { items: items.to_vec(), sized }
    }
}

impl<T> std::str::FromStr for Stack<T> {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        Ok(Stack {
            items: Vec::new(),
            sized: text.len() * mem::size_of::<T>(),
        })
    }
}

fn copied<T: Clone>(items: &[T]) -> Stack<T> {
    Stack::from(items)
}

fn parsed<T>(text: &str) -> Stack<T> {
    text.parse().unwrap()
}

fn main() {
    let s: Stack<u8> = [1, 2, 3].into_iter().collect();
    let u: Stack<()> = [(), ()].into_iter().collect();
    println!("{} {} {} {}", s.items.len(), s.sized, u.items.len(), u.sized);
    let (mut s, mut u) = (s.clone(), u.clone());
    fill(&mut s, 4);
    fill(&mut u, ());
    println!("{} {} {} {}", s.items.len(), s.sized, u.items.len(), u.sized);
    let (a, b) = (copied(&[1u32, 2]), copied(&[(), ()]));
    let (c, d) = (parsed::<u16>("abc"), parsed::<()>("abc"));
    println!("{} {} {} {}", a.sized, b.sized, c.sized, d.sized);
}
