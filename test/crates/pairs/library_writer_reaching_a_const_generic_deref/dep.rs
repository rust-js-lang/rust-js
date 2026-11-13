//! A library's writer of a const-generic type, `Stack<T, CAP>`'s, that
//! calls a trait's method of what still holds `CAP`, `Guard<'_, T, CAP>`'s
//! `deref_mut`: a library asks what each writer may run (#1608), which
//! selects no impl where the const's type isn't known, as rustc crashed
//! doing, `cannot find CAP in param-env`.
use std::fmt;
use std::ops::{Deref, DerefMut};

pub struct Stack<T, const CAP: usize> {
    items: Vec<T>,
}

pub struct Guard<'a, T, const CAP: usize> {
    stack: &'a mut Stack<T, CAP>,
}

impl<'a, T, const CAP: usize> Deref for Guard<'a, T, CAP> {
    type Target = Stack<T, CAP>;
    fn deref(&self) -> &Stack<T, CAP> {
        self.stack
    }
}

impl<'a, T, const CAP: usize> DerefMut for Guard<'a, T, CAP> {
    fn deref_mut(&mut self) -> &mut Stack<T, CAP> {
        self.stack
    }
}

impl<T: fmt::Debug + Clone, const CAP: usize> fmt::Debug for Stack<T, CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut copy = Stack::<T, CAP> { items: self.items.clone() };
        let mut guard = Guard { stack: &mut copy };
        let len = guard.deref_mut().items.len();
        write!(f, "Stack({len} of {CAP})")
    }
}

pub fn stack<const CAP: usize>(items: Vec<u8>) -> Stack<u8, CAP> {
    Stack { items }
}
