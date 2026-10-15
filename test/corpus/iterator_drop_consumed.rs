// An iterator of the crate's with a destructor, consumed by value,
// `collect()`, `count()`: dropped after, as the method's end drops it,
// arrayvec's `Drain`, which moves its vector's tail back (ADR 0098).
struct Draining<'a> {
    items: &'a mut Vec<i32>,
    at: usize,
}

impl Iterator for Draining<'_> {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        let item = self.items.get(self.at).copied();
        self.at += 1;
        item
    }
}

impl Drop for Draining<'_> {
    fn drop(&mut self) {
        println!("dropped after {}", self.at);
        self.items.clear();
    }
}

fn main() {
    let mut v = vec![1, 2, 3];
    let taken: Vec<i32> = Draining { items: &mut v, at: 0 }.collect();
    println!("{taken:?} {v:?}");
    let mut w = vec![4, 5];
    let n = Draining { items: &mut w, at: 0 }.count();
    println!("{n} {w:?}");
}
