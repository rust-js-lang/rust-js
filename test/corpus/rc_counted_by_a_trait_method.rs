// A trait impl's generic method that reads an `Rc<T>`'s count is called
// through its trait, a dictionary, by callers its own calls don't name:
// so its `T` may be any type, and every `Rc` is counted, the one given it
// through generic code too (#1001).
use std::rc::Rc;

trait Counts {
    fn count_of<T>(&self, rc: &Rc<T>) -> usize;
}

struct Counter;

impl Counts for Counter {
    fn count_of<T>(&self, rc: &Rc<T>) -> usize {
        Rc::strong_count(rc)
    }
}

struct Item {
    id: u32,
}

fn through<C: Counts>(counter: &C, rc: &Rc<Item>) -> usize {
    counter.count_of(rc)
}

fn main() {
    let item = Rc::new(Item { id: 7 });
    let copies = [Rc::clone(&item), Rc::clone(&item)];
    println!("{} {}", through(&Counter, &item), copies.len());
    drop(copies);
    println!("{} {}", through(&Counter, &item), item.id);
}
