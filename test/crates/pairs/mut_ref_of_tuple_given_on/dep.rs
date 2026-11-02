//! A `&mut` a method returns in a tuple, given on to a guard that writes
//! through it as it's dropped, as smallvec's `extend` gives `triple_mut()`'s
//! `len_ptr` to its `SetLenOnDrop` (#476).
pub struct Stack<T> {
    items: Vec<T>,
    len: usize,
}

struct SetLenOnDrop<'a> {
    len: &'a mut usize,
    local_len: usize,
}

impl<'a> SetLenOnDrop<'a> {
    fn new(len: &'a mut usize) -> Self {
        let local_len = *len;
        SetLenOnDrop { len, local_len }
    }

    fn increment(&mut self) {
        self.local_len += 1;
    }
}

impl Drop for SetLenOnDrop<'_> {
    fn drop(&mut self) {
        *self.len = self.local_len;
    }
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack { items: Vec::new(), len: 0 }
    }

    fn triple_mut(&mut self) -> (&mut Vec<T>, &mut usize, usize) {
        let cap = self.items.capacity();
        (&mut self.items, &mut self.len, cap)
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iterable: I) {
        let (items, len_ptr, _cap) = self.triple_mut();
        let mut len = SetLenOnDrop::new(len_ptr);
        for item in iterable {
            items.push(item);
            len.increment();
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}
