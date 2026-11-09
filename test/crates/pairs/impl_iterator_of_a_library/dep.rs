//! A library's functions giving back what an `impl DoubleEndedIterator`
//! hides: a peeked `peekable()`, a `map()` and its own iterator, each of
//! the app's items with a destructor (#1142).
pub struct Queue<T>(pub Vec<T>);

impl<T> Iterator for Queue<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}

impl<T> DoubleEndedIterator for Queue<T> {
    fn next_back(&mut self) -> Option<T> {
        self.0.pop()
    }
}

impl<T> ExactSizeIterator for Queue<T> {}

pub fn peeked<T>(items: Vec<T>) -> impl DoubleEndedIterator<Item = T> + ExactSizeIterator {
    let mut it = items.into_iter().peekable();
    it.peek();
    it
}

pub fn mapped<T, U>(items: Vec<T>, f: fn(T) -> U) -> impl DoubleEndedIterator<Item = U> + ExactSizeIterator {
    items.into_iter().map(f)
}

pub fn queue<T>(items: Vec<T>) -> impl DoubleEndedIterator<Item = T> + ExactSizeIterator {
    Queue(items)
}
