//! A generic impl's constant of its const parameter, `CAPACITY = CAP`, read
//! by the trait's provided method, as arrayvec's `ArrayVecImpl` reads its
//! own (ADR 0176, amended).
trait Bounded {
    type Item;
    const CAPACITY: usize;

    fn items(&self) -> &[Self::Item];

    fn room(&self) -> usize {
        Self::CAPACITY - self.items().len()
    }
}

pub struct Stack<T, const CAP: usize> {
    items: Vec<T>,
}

impl<T, const CAP: usize> Bounded for Stack<T, CAP> {
    type Item = T;
    const CAPACITY: usize = CAP;

    fn items(&self) -> &[T] {
        &self.items
    }
}

impl<T, const CAP: usize> Stack<T, CAP> {
    pub fn new() -> Self {
        Stack { items: Vec::new() }
    }

    /// The item back where the stack is full.
    pub fn try_push(&mut self, item: T) -> Result<(), T> {
        match Bounded::room(self) {
            0 => Err(item),
            _ => {
                self.items.push(item);
                Ok(())
            }
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<T, const CAP: usize> Default for Stack<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}
