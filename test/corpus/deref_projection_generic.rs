//@ library-differs
// A method's result of a projection, `DerefMut`'s `&mut Self::Target`, of
// an impl whose parameters a caller's numbers otherwise, a guard's of
// `impl<'a, T, const CAP: usize>`, as arrayvec's `Drain`: asked of in its
// own environment, where rustc crashed asked in the caller's.
use std::ops::{Deref, DerefMut};

struct Stack<T, const CAP: usize> {
    xs: [T; CAP],
}

impl<T, const CAP: usize> Deref for Stack<T, CAP> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.xs
    }
}

impl<T, const CAP: usize> DerefMut for Stack<T, CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.xs
    }
}

struct Guard<'a, T, const CAP: usize> {
    stack: &'a mut Stack<T, CAP>,
}

impl<'a, T, const CAP: usize> Guard<'a, T, CAP> {
    fn len(&mut self) -> usize {
        self.stack.deref_mut().len()
    }
}

fn main() {
    let mut s = Stack { xs: [1, 2, 3] };
    let mut g = Guard { stack: &mut s };
    println!("{}", g.len());
}
