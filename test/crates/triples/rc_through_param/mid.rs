// A crate's generic bag, cloning what it holds (ADR 0523).
pub struct Bag<T> {
    pub items: Vec<T>,
}

impl<T: Clone> Bag<T> {
    pub fn dup_first(&mut self) {
        let first = self.items[0].clone();
        self.items.push(first);
    }
}
