//! A library's iterator whose `next` grows a `Vec` by
//! `extend(it.by_ref().take(1))`, which asks whether the `I` it holds is
//! `TrustedLen`, and its generic functions given one: `last_of_pool` gives
//! it where a generic iterator goes, `fold_pool` folds it through a `&mut`.
//! A consumer giving them items with a destructor compiles its own of each
//! from their MIR (ADR 0914), and asks its callers what `I` is (#2327).
pub struct Pool<I: Iterator> {
    pub it: I,
    pub items: Vec<I::Item>,
}

impl<I: Iterator> Iterator for Pool<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.items.extend(self.it.by_ref().take(1));
        self.items.pop()
    }
}

fn last_of<J: Iterator>(items: J) -> Option<J::Item> {
    items.last()
}

pub fn last_of_pool<I: Iterator>(pool: Pool<I>) -> Option<I::Item> {
    last_of(pool)
}

pub fn fold_pool<I: Iterator>(mut pool: Pool<I>) -> usize {
    pool.by_ref().fold(0, |n, _| n + 1)
}
